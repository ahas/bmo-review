import type { Commit, FileSource, Page, Project, TreeEntry } from "../../shared/types";

// A stand-in of the review service for the browser tests: the reads the code review screen makes on opening a file,
// answered from fixed rows under the screen's prefix, and an event stream that stays open and says nothing. It keeps
// nothing and takes no write; every path it does not know answers 404 as the service would.

export const PROJECT = "sample";
export const FILE = "main.rs";

const HEAD = "0123456789abcdef0123456789abcdef01234567";
const AT = Date.UTC(2030, 4, 1);
const TEXT = ["fn main() {", "    let answer = 42;", "    println!(\"{answer}\");", "}", ""].join("\n");

const PROJECTS: Project[] = [{ name: PROJECT, checkout: "sample", state: "ready", head: HEAD, branch: "main", uncommittedCount: 0 }];
const TREE: TreeEntry[] = [{ path: FILE, kind: "file", byteCount: TEXT.length, pinnedRevision: null }];
const SOURCE: FileSource = { path: FILE, revision: HEAD, byteCount: TEXT.length, text: TEXT, lineCount: 4, cut: false };
const COMMITS: Commit[] = [{ id: HEAD, shortId: HEAD.slice(0, 7), subject: "Start the sample", authorName: "tester", at: AT, files: [], diff: null, cut: false }];

function page<T>(items: T[]): Page<T> {
  return { items, more: false };
}

function answer(path: string): unknown {
  const project = `/screen/projects/${PROJECT}`;

  if (path === "/screen/projects") return page(PROJECTS);
  if (path === `${project}/tree`) return page(TREE);
  if (path === `${project}/files/${FILE}`) return SOURCE;
  if (path === `${project}/commits`) return page(COMMITS);
  if (path === `${project}/matches` || path === `${project}/definitions`) return page([]);
  if (path === "/screen/reviews") return page([]);

  return undefined;
}

/** Starts the stand-in on a free loopback port; `stop` takes it down with every stream it holds open. */
export function standIn(): { address: string; stop: () => void } {
  const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    // The event stream is held open for as long as the screen reads it.
    idleTimeout: 0,
    fetch(request) {
      const path = new URL(request.url).pathname;

      if (request.method === "GET" && path === "/screen/events") {
        return new Response(new ReadableStream({ start: (stream) => stream.enqueue(new TextEncoder().encode(": open\n\n")) }), {
          headers: { "content-type": "text/event-stream", "cache-control": "no-cache" }
        });
      }

      const read = request.method === "GET" ? answer(path) : undefined;

      return read === undefined ? Response.json({ code: "not-found", error: "Not found." }, { status: 404 }) : Response.json(read);
    }
  });

  return { address: `http://127.0.0.1:${server.port}`, stop: () => void server.stop(true) };
}
