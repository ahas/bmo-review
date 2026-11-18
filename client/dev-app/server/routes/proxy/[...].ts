// The dev relay: /proxy/<service>/<path...> forwarded to the review service at the address the environment names
// (`BMO_DEV_REVIEW_ADDRESS`), with the credential it names (`BMO_DEV_REVIEW_TOKEN`) where it names one. Nothing of a
// deployment's tokens stands here; the values come from the environment the host runs with.

const ADDRESS = process.env.BMO_DEV_REVIEW_ADDRESS ?? "http://127.0.0.1:13140";
const TOKEN = process.env.BMO_DEV_REVIEW_TOKEN ?? "";

export default defineEventHandler(async (event) => {
  const rest = event.path.replace(/^\/proxy\/[^/]+/, "");
  const type = event.headers.get("content-type");
  const sent = event.method === "GET" || event.method === "HEAD" ? undefined : await readRawBody(event, false);

  const response = await fetch(`${ADDRESS}${rest}`, {
    method: event.method,
    headers: {
      ...(TOKEN === "" ? {} : { authorization: `Bearer ${TOKEN}` }),
      ...(type === null ? {} : { "content-type": type })
    },
    // The body rides through as the bytes the caller sent, as the console's relay does.
    body: sent === undefined ? undefined : new Uint8Array(sent)
  });

  const answered = response.headers.get("content-type") ?? "application/json";

  // The event stream rides through open, as the console relays it.
  if (answered.includes("text/event-stream")) return new Response(response.body, { status: response.status, headers: { "content-type": answered } });

  return new Response(await response.arrayBuffer(), { status: response.status, headers: { "content-type": answered } });
});
