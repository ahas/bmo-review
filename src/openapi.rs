//! The service's OpenAPI 3.1 document (`docs/api.md`): every registered route as it states itself, with its
//! parameters, its body, its answer and the codes it can answer, the bodies' schemas written from the types the
//! routes take and answer, the events, and the harness plugin's tools from the table the MCP server lists. The
//! repository keeps it as `docs/openapi.json`, which `bmo-review openapi` prints.

use schemars::JsonSchema;
use schemars::generate::SchemaGenerator;
use serde_json::{Map, Value, json};

use crate::console;
use crate::events::{COMMENT_CREATED, FILING_CREATED, REVIEW_CHANGED, REVIEW_CREATED};
use crate::http::routes;
use crate::mcp::{ENTRY, MCP, PLUGIN, generator, tools};
use crate::wire::WireError;

/// Where the schemas the routes refer to stand in the document.
const SCHEMAS: &str = "/components/schemas";

const DESCRIPTION: &str = "Code review over git checkouts: projects named as the graph service names them, \
  their trees, files and commits at a revision, each commit's diff, content and definition searches, reviews of \
  a revision with their comments on its lines, files and folders, and items a review files into the job service \
  through its client. Who wrote a review or a comment is an opaque authorName. Bodies are JSON with camelCase \
  keys; times are integers of epoch milliseconds. A list is a page, `after` being the cursor each route states. \
  A revision is seven to sixty-four hex characters, HEAD where absent. A file's path stands as the rest of the \
  path. Every failure is answered in the error shape with its status. Every route takes the service token; \
  where a console face stands, every route is answered again under `/screen`, the console's prefix, to the \
  console credential alone, refused 403 `forbidden` unless its `Owner-Role` opens the screen, and the console \
  face's manifest and files, by bmo-ui's console plugin contract, are stated under the `console` tag. The \
  harness plugin's tools, each a thin call of one route, are stated under `x-tools`.";

/// Each code a route can answer, its status and what it means.
const CODES: &[(&str, u16, &str)] = &[
  ("bad-request", 400, "the body, the query or the path is not the shape the route takes"),
  ("unauthorized", 401, "the bearer token is missing or wrong"),
  (
    "forbidden",
    403,
    "under /screen, or for the console bundle's module and stylesheet, the console's call names no role that \
     opens the screen",
  ),
  ("no-route", 404, "no route answers the method and path"),
  (
    "not-found",
    404,
    "nothing stands at the project, the revision, the path, the review or the console bundle's file the request \
     names",
  ),
  ("conflict", 409, "the review's states do not allow the change"),
  ("gone", 410, "the events since the number asked for are no longer kept"),
  (
    "unreadable-checkout",
    422,
    "the project's checkout is missing, in no git repository or without commits",
  ),
  ("unknown-project", 422, "a review names a project the graph service does not know"),
  (
    "unknown-revision",
    422,
    "a review names a revision that resolves to no commit of the project",
  ),
  (
    "no-such-place",
    422,
    "a comment's place is no file or folder of the revision, or its lines lie off the file",
  ),
  ("not-a-name", 422, "the definition search does not take the word"),
  ("filing-refused", 422, "the job service refused the item"),
  ("failed", 500, "the service failed"),
  (
    "upstream-failed",
    502,
    "the graph service or the job service could not be reached or failed",
  ),
];

/// The codes every route reading a project can answer.
pub const READING_CODES: &[&str] = &["not-found", "unreadable-checkout", "upstream-failed"];

/// One route as the document states it, which the route gives beside its answer.
pub struct Stated {
  pub id: &'static str,
  pub tag: &'static str,
  pub summary: &'static str,
  pub parameters: Vec<Value>,
  pub body: Option<Value>,
  /// The success's status and content.
  pub answer: (u16, Value),
  /// The codes of its own, beside the token's and those every route can answer.
  pub codes: Vec<&'static str>,
}

impl Stated {
  fn operation(self) -> Value {
    let mut codes = vec!["unauthorized"];

    if !self.parameters.is_empty() || self.body.is_some() {
      codes.push("bad-request");
    }

    codes.extend(self.codes);
    codes.push("failed");

    let mut responses = Map::new();
    let (status, content) = self.answer;

    responses.insert(status.to_string(), json!({ "description": self.summary, "content": content }));
    responses.insert("default".into(), json!({ "$ref": "#/components/responses/Error" }));

    let mut operation = json!({
      "operationId": self.id,
      "tags": [self.tag],
      "summary": self.summary,
      "responses": responses,
      "x-codes": codes,
    });

    if !self.parameters.is_empty() {
      operation["parameters"] = self.parameters.into();
    }

    if let Some(body) = self.body {
      operation["requestBody"] =
        json!({ "required": true, "content": { "application/json": { "schema": body } } });
    }

    operation
  }
}

/// A JSON answer of type `T`.
pub fn json_of<T: JsonSchema>(generator: &mut SchemaGenerator) -> Value {
  json!({ "application/json": { "schema": generator.subschema_for::<T>() } })
}

/// A body of type `T`.
pub fn schema_of<T: JsonSchema>(generator: &mut SchemaGenerator) -> Value {
  generator.subschema_for::<T>().to_value()
}

/// The query parameters of a query type, each property one parameter.
pub fn query<Query: JsonSchema>(generator: &mut SchemaGenerator) -> Vec<Value> {
  let schema = Query::json_schema(generator).to_value();
  let Some(properties) = schema["properties"].as_object() else {
    return Vec::new();
  };

  properties
    .iter()
    .map(|(name, property)| {
      let mut parameter = json!({ "name": name, "in": "query", "required": false, "schema": property });

      if let Some(description) = property.get("description") {
        parameter["description"] = description.clone();
      }

      parameter
    })
    .collect()
}

fn key(name: &str, description: &str, schema: Value) -> Value {
  json!({ "name": name, "in": "path", "required": true, "description": description, "schema": schema })
}

/// The path's project.
pub fn project_key() -> Value {
  key(
    "projectName",
    "The name the graph service registers the project under.",
    json!({ "type": "string" }),
  )
}

/// The path's review.
pub fn review_key() -> Value {
  key("reviewId", "The review's id.", json!({ "type": "integer", "format": "int64" }))
}

/// The path's file, the rest of the path.
pub fn file_key() -> Value {
  key(
    "filePath",
    "The file's path from the repository's root, its slashes standing as they are.",
    json!({ "type": "string" }),
  )
}

/// The path's commit.
pub fn commit_key() -> Value {
  key(
    "commitId",
    "The commit, seven to sixty-four hex characters.",
    json!({ "type": "string" }),
  )
}

/// The OpenAPI document.
pub fn openapi() -> Value {
  let mut generator = generator(SCHEMAS);
  let mut registered: Vec<_> = routes().collect();
  let mut paths = Map::new();
  let mut tags: Vec<Value> = Vec::new();

  registered.sort_by_key(|route| (route.path(), route.method().to_string()));

  for route in registered {
    // The router writes the rest of a path as `{*name}`; the document writes every parameter as `{name}`.
    let path = route.path().replace("{*", "{");
    let method = route.method().as_str().to_lowercase();
    let stated = route.stated(&mut generator);
    let tag = json!({ "name": stated.tag });

    if !tags.contains(&tag) {
      tags.push(tag);
    }

    paths.entry(path).or_insert_with(|| json!({}))[method] = stated.operation();
  }

  for (path, stated) in console::stated() {
    let tag = json!({ "name": stated.tag });

    if !tags.contains(&tag) {
      tags.push(tag);
    }

    paths.entry(path).or_insert_with(|| json!({}))["get"] = stated.operation();
  }

  let x_tools: Vec<Value> = tools()
    .iter()
    .map(|tool| {
      json!({
        "name": tool.name,
        "description": tool.description,
        "input": tool.input(&mut generator),
        "output": tool.output(&mut generator),
        "route": tool.route,
      })
    })
    .collect();
  let error = generator.subschema_for::<WireError>();
  let codes: Vec<String> = CODES
    .iter()
    .map(|(code, status, meaning)| format!("- `{code}` ({status}): {meaning}"))
    .collect();

  json!({
    "openapi": "3.1.0",
    "info": { "title": "bmo-review", "version": env!("CARGO_PKG_VERSION"), "description": DESCRIPTION },
    "servers": [{ "url": "/", "description": "the review service" }],
    "security": [{ "bearer": [] }],
    "x-layer": "upstream",
    "x-stage": 3,
    "x-calls": ["job", "graph"],
    "x-harness": { "plugin": PLUGIN, "mcp": MCP, "entry": ENTRY },
    "tags": tags,
    "paths": paths,
    "components": {
      "schemas": generator.take_definitions(true),
      "responses": {
        "Error": {
          "description": format!("A failure, in the common error shape. The codes:\n{}", codes.join("\n")),
          "content": { "application/json": { "schema": error } },
        },
      },
      "securitySchemes": {
        "bearer": {
          "type": "http",
          "scheme": "bearer",
          "description": "The service token, from the caller's own configuration; under /screen and on the console \
            face's routes, the console's credential alone.",
        },
      },
    },
    "x-events": [REVIEW_CREATED, REVIEW_CHANGED, COMMENT_CREATED, FILING_CREATED],
    "x-tools": x_tools,
  })
}

/// The document as the repository keeps it: two-space indented JSON with a closing newline.
pub fn openapi_json() -> String {
  let mut text = serde_json::to_string_pretty(&openapi()).unwrap_or_default();

  text.push('\n');
  text
}
