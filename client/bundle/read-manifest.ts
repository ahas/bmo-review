import { consoleManifest } from "bmo-ui/shared/console-manifest";

// A manifest read through bmo-ui's reading of the console plugin contract, as the service's own test asks it
// (docs/api.md, 「The console bundle」): the manifest on standard input, the manifest as a console reads it on
// standard output, and a manifest that breaks the contract refused with each field at fault named and an exit of
// 1.
let given = "";

for await (const part of process.stdin) {
  given += String(part);
}

try {
  process.stdout.write(JSON.stringify(consoleManifest(JSON.parse(given))));
} catch (fault) {
  process.stderr.write(`${fault instanceof Error ? fault.message : String(fault)}\n`);
  process.exitCode = 1;
}
