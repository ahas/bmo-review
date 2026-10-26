import "./screen.css";

import { callUnder } from "../app/domain/review";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";

// The served module's entry (docs/api.md, 「The console bundle」): the screen's one route under the manifest's
// screen path, its page loaded once a reader first opens it, and every call the screen makes put under the
// service's screen prefix, which the manifest names as its API prefix and where the service guards the reader's
// role.
callUnder("/screen");

const screen: ConsoleBundleModule = {
  // The route asks for the console's title bar (bmo-ui's `docs/console-bundles.md`, the title bar).
  routes: [{ path: "/code", name: "code", component: () => import("../app/pages/code.vue"), meta: { consoleTitleBar: true } }]
};

export default screen;
