//! `bmo-review mcp`: the harness plugin's MCP server on standard input and output (`docs/api.md`, 「The harness
//! plugin」). Each tool is a thin call of one route through the service's own client; the server lists them and
//! the OpenAPI document states them under `x-tools`, both from the one table here.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use rmcp::model::{
  CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation, InitializeResult,
  JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use schemars::generate::{SchemaGenerator, SchemaSettings};
use schemars::{JsonSchema, Schema};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::client::{ClientFailure, ReviewClient};
use crate::wire::{
  CommentQuery, CommitQuery, NewComment, NewFiling, NewReview, ProjectQuery, ReviewQuery, ReviewThread,
};

/// The plugin a harness takes the review service in, and the command that starts its MCP server.
pub const PLUGIN: &str = "bmo-review";
pub const MCP: &str = "bmo-review mcp";

/// What a harness does first with it, which the server tells a harness when it starts.
pub const ENTRY: &str = "A harness reads a change with review_diff, writes its findings with review_create and \
  review_comment, and files a follow-up item with review_file_item.";

/// What a tool's call came to short of the route's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolFailure {
  /// The arguments are not the shape the tool takes.
  Arguments(String),
  /// The call reached no answer of the route's shape.
  Client(ClientFailure),
}

impl ToolFailure {
  /// The failure as the tool error's content: the error shape the service answered with and its status, or the
  /// sentence saying why nothing answered.
  pub fn content(&self) -> Value {
    match self {
      Self::Arguments(why) => json!({ "code": "bad-request", "error": why }),
      Self::Client(ClientFailure::Answered {
        status,
        answer: Some(answer),
        ..
      }) => json!({ "status": status, "code": answer.code, "error": answer.error }),
      Self::Client(failure @ ClientFailure::Answered { status, .. }) => {
        json!({ "status": status, "error": failure.to_string() })
      }
      Self::Client(failure) => json!({ "error": failure.to_string() }),
    }
  }
}

type Answering = Pin<Box<dyn Future<Output = Result<Value, ToolFailure>> + Send>>;
type Call = Box<dyn Fn(ReviewClient, Value) -> Answering + Send + Sync>;

/// One tool: its name, what it does, the route it calls, its input and output, and the call itself.
pub struct ReviewTool {
  pub name: &'static str,
  pub description: &'static str,
  /// The routes the tool calls, as their method and path.
  pub route: &'static str,
  input: fn(&mut SchemaGenerator) -> Schema,
  output: fn(&mut SchemaGenerator) -> Schema,
  call: Call,
}

impl ReviewTool {
  /// The tool's input, its nested types referred to where `generator` keeps its definitions.
  pub fn input(&self, generator: &mut SchemaGenerator) -> Schema {
    (self.input)(generator)
  }

  /// The tool's output, referred to where `generator` keeps its definitions.
  pub fn output(&self, generator: &mut SchemaGenerator) -> Schema {
    (self.output)(generator)
  }

  /// Calls the tool's route through `client` with the tool's arguments.
  pub async fn call(&self, client: ReviewClient, arguments: Value) -> Result<Value, ToolFailure> {
    (self.call)(client, arguments).await
  }
}

/// A tool whose input is `I` and whose answer is `O`, calling `call` with the arguments read as `I`.
fn tool<I, O, F, Answer>(
  name: &'static str,
  description: &'static str,
  route: &'static str,
  call: F,
) -> ReviewTool
where
  I: DeserializeOwned + JsonSchema,
  O: Serialize + JsonSchema,
  F: Fn(ReviewClient, I) -> Answer + Send + Sync + 'static,
  Answer: Future<Output = Result<O, ClientFailure>> + Send + 'static,
{
  ReviewTool {
    name,
    description,
    route,
    input: I::json_schema,
    output: SchemaGenerator::subschema_for::<O>,
    call: Box::new(move |client, arguments| {
      let pending =
        serde_json::from_value::<I>(arguments)
          .map(|input| call(client, input))
          .map_err(|error| {
            ToolFailure::Arguments(format!("The arguments are not the shape this tool takes: {error}."))
          });

      Box::pin(async move {
        let answer = pending?.await.map_err(ToolFailure::Client)?;

        serde_json::to_value(answer).map_err(|why| ToolFailure::Arguments(why.to_string()))
      })
    }),
  }
}

/// A project's commits, newest first.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commits {
  project_name: String,
  /// The revision the list walks back from; HEAD where absent.
  #[serde(default)]
  to: Option<String>,
  /// An older revision the list stops short of, with its history.
  #[serde(default)]
  from: Option<String>,
  /// Only the commits that touched this path.
  #[serde(default)]
  path: Option<String>,
  /// The full hash of the last commit read; the page starts past it.
  #[serde(default)]
  after: Option<String>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default)]
  limit: Option<u32>,
}

/// One commit of a project.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Diff {
  project_name: String,
  /// The commit, seven to sixty-four hex characters.
  commit_id: String,
}

/// One review, and a page of its comments.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Thread {
  review_id: i64,
  /// The id of the last comment read; the comments continue with newer ones.
  #[serde(default)]
  after: Option<i64>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default)]
  limit: Option<u32>,
}

/// A comment to write on a review.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commenting {
  review_id: i64,
  /// A file or folder of the review's revision; the whole review where absent or null.
  #[serde(default)]
  path: Option<String>,
  /// The first line it points at, from 1, with a file's path.
  #[serde(default)]
  line: Option<u32>,
  /// The last line of a range, past `line`.
  #[serde(default)]
  end_line: Option<u32>,
  text: String,
  author_name: String,
}

/// An item to file from a review into the job service.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Filing {
  review_id: i64,
  title: String,
  brief: String,
  /// The job service's queue the item goes into.
  queue_name: String,
}

/// The eight tools.
pub fn tools() -> Vec<ReviewTool> {
  vec![
    tool(
      "review_projects",
      "The projects that can be read: each repository agency keeps on this machine, with its state and HEAD.",
      "GET /projects",
      |client, query: ProjectQuery| async move { client.projects(&query).await },
    ),
    tool(
      "review_commits",
      "Commits, newest first, from a revision or between two, each with the files it touched.",
      "GET /projects/{projectName}/commits",
      |client, asked: Commits| async move {
        let query = CommitQuery {
          to: asked.to,
          from: asked.from,
          path: asked.path,
          after: asked.after,
          limit: asked.limit,
        };

        client.commits(&asked.project_name, &query).await
      },
    ),
    tool(
      "review_diff",
      "One commit and its diff against its first parent, the excluded files left out.",
      "GET /projects/{projectName}/commits/{commitId}",
      |client, asked: Diff| async move { client.commit(&asked.project_name, &asked.commit_id).await },
    ),
    tool(
      "review_reviews",
      "Reviews by project, revision and state, newest first.",
      "GET /reviews",
      |client, query: ReviewQuery| async move { client.reviews(&query).await },
    ),
    tool(
      "review_create",
      "Open a review of a revision of a project, under a title and the author's name.",
      "POST /reviews",
      |client, review: NewReview| async move { client.create_review(&review).await },
    ),
    tool(
      "review_review",
      "One review and its thread: the review, and a page of its comments, oldest first.",
      "GET /reviews/{reviewId}, GET /reviews/{reviewId}/comments",
      |client, asked: Thread| async move {
        let review = client.review(asked.review_id).await?;
        let query = CommentQuery {
          after: asked.after,
          limit: asked.limit,
        };
        let comments = client.comments(asked.review_id, &query).await?;

        Ok(ReviewThread { review, comments })
      },
    ),
    tool(
      "review_comment",
      "Comment on a review: on the whole, a folder, a file, a line, or a range of lines of its revision.",
      "POST /reviews/{reviewId}/comments",
      |client, asked: Commenting| async move {
        let comment = NewComment {
          path: asked.path,
          line: asked.line,
          end_line: asked.end_line,
          text: asked.text,
          author_name: asked.author_name,
        };

        client.create_comment(asked.review_id, &comment).await
      },
    ),
    tool(
      "review_file_item",
      "File an item from a review into the job service, in the queue named, with a title and a brief.",
      "POST /reviews/{reviewId}/filings",
      |client, asked: Filing| async move {
        let filing = NewFiling {
          title: asked.title,
          brief: asked.brief,
          queue_name: asked.queue_name,
        };

        client.create_filing(asked.review_id, &filing).await
      },
    ),
  ]
}

/// The generator every tool schema and the OpenAPI document are written with: JSON Schema 2020-12, read as a
/// caller sends it, definitions kept at `definitions_path`.
pub fn generator(definitions_path: &'static str) -> SchemaGenerator {
  SchemaSettings::draft2020_12()
    .for_deserialize()
    .with(|settings| {
      settings.definitions_path = definitions_path.into();
      settings.meta_schema = None;
    })
    .into_generator()
}

/// The server over the service the client reaches.
pub struct ReviewServer {
  client: ReviewClient,
  tools: Arc<Vec<ReviewTool>>,
}

impl ReviewServer {
  pub fn new(client: ReviewClient) -> Self {
    Self {
      client,
      tools: Arc::new(tools()),
    }
  }

  /// The tools as MCP lists them, each input and output a schema of its own with its definitions inside.
  pub fn listed(&self) -> Vec<Tool> {
    self.tools.iter().map(listed).collect()
  }
}

fn listed(tool: &ReviewTool) -> Tool {
  let mut generator = generator("/$defs");
  let input = tool.input(&mut generator);
  let output = tool.output(&mut generator);
  let definitions = generator.take_definitions(true);
  let mut listed = Tool::new(tool.name, tool.description, standing_alone(input, &definitions));

  listed.output_schema = Some(Arc::new(standing_alone(output, &definitions)));
  listed
}

/// A schema made to stand on its own: an object at its root, with every definition it refers to beside it.
fn standing_alone(schema: Schema, definitions: &Map<String, Value>) -> JsonObject {
  let mut object = match schema.to_value() {
    Value::Object(object) if object.contains_key("$ref") => {
      let mut wrapped = Map::new();

      wrapped.insert("allOf".into(), Value::Array(vec![Value::Object(object)]));
      wrapped.insert("type".into(), "object".into());
      wrapped
    }
    Value::Object(object) => object,
    _ => Map::from_iter([("type".to_owned(), Value::from("object"))]),
  };

  if !definitions.is_empty() {
    object.insert("$defs".into(), Value::Object(definitions.clone()));
  }

  object
}

impl ServerHandler for ReviewServer {
  fn get_info(&self) -> InitializeResult {
    let mut info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build());
    let mut server = Implementation::default();

    server.name = "bmo-review".to_owned();
    server.version = env!("CARGO_PKG_VERSION").to_owned();
    info.server_info = server;
    info.instructions = Some(ENTRY.to_owned());
    info
  }

  async fn list_tools(
    &self,
    _: Option<PaginatedRequestParams>,
    _: RequestContext<RoleServer>,
  ) -> Result<ListToolsResult, ErrorData> {
    Ok(ListToolsResult::with_all_items(self.listed()))
  }

  async fn call_tool(
    &self,
    request: CallToolRequestParams,
    _: RequestContext<RoleServer>,
  ) -> Result<CallToolResponse, ErrorData> {
    let Some(tool) = self.tools.iter().find(|tool| tool.name == request.name) else {
      return Err(ErrorData::invalid_params(format!("There is no tool {}.", request.name), None));
    };
    let arguments = Value::Object(request.arguments.unwrap_or_default());
    let result = match tool.call(self.client.clone(), arguments).await {
      Ok(answer) => CallToolResult::structured(answer),
      Err(failure) => CallToolResult::error(vec![ContentBlock::text(failure.content().to_string())]),
    };

    Ok(CallToolResponse::Complete(result))
  }
}

/// The MCP server, run on standard input and output until its harness closes them.
pub async fn serve(client: ReviewClient) -> Result<(), String> {
  let running = rmcp::ServiceExt::serve(ReviewServer::new(client), rmcp::transport::stdio())
    .await
    .map_err(|why| format!("the MCP session did not start: {why}"))?;

  running
    .waiting()
    .await
    .map(|_| ())
    .map_err(|why| format!("the MCP session ended in a failure: {why}"))
}
