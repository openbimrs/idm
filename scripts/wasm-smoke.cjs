// Round-trip smoke test for the wasm binding. Usage: node wasm-smoke.cjs <glue-dir>
const assert = require("node:assert/strict");
const path = require("node:path");

const idmxml = require(path.join(process.argv[2], "openbim_idm.js"));
const { Document } = idmxml;

// Nothing may leak onto globalThis; the module itself is the namespace.
assert.equal(typeof globalThis.Document, "undefined");

const document = Document.create("Coordination", "IDM-001");
assert.equal(document.rootName, "idm");
assert.deepEqual(JSON.parse(document.validateJson()), []);

const node = JSON.parse(document.nodeJson("/idm/specId"));
assert.equal(node.handle, "specId");

// Edits are reversible.
const original = document.copy();
const inverse = document.applyJson(
  JSON.stringify({ op: "set_attribute", path: "/idm/specId", name: "fullTitle", value: "X" }),
);
assert.equal(document.attribute("/idm/specId", "fullTitle"), "X");
assert.ok(!document.equals(original));
document.applyJson(inverse);
assert.ok(document.equals(original));

// Structural editing and locators.
document.appendSchemaChild("/idm/er", "informationUnit");
const id = document.attribute("/idm/er/informationUnit[1]", "id");
document.insertSchemaChild("/idm/er", "informationUnit", 0);
assert.equal(document.pathOf(`id:${id}`), "/idm/er[0]/informationUnit[2]");
assert.deepEqual(JSON.parse(document.validateJson()), []);

// XML / bytes round trip, including a non-UTF-8 encoding.
const reparsed = Document.parse(document.toXml(false));
assert.ok(reparsed.equals(Document.parse(document.toXml(false))));
const bytes = Document.create("Grüße", "C").toBytes(false, "utf-16le");
const loaded = Document.parseBytes(bytes);
assert.equal(loaded.document.attribute("/idm/specId", "fullTitle"), "Grüße");
assert.equal(JSON.parse(loaded.metadataJson).encoding, "utf-16le");

// Errors are Error objects with a stable code.
assert.throws(() => document.text("/idm/missing"), (error) => {
  assert.ok(error instanceof Error);
  assert.equal(error.code, "path_not_found");
  return true;
});
assert.throws(() => document.setText("/idm/uc", "x"), (error) => error.code === "content_model");

assert.ok(JSON.parse(idmxml.schemaCatalogJson()).element_names.length > 0);
console.log(`wasm smoke test passed (engine ${idmxml.engineVersion()})`);
