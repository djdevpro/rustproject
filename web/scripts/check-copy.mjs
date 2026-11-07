import fs from "node:fs";
import ts from "typescript";
const dictionaries = JSON.parse(fs.readFileSync("src/native-en.json", "utf8"));
const source = ts.createSourceFile(
  "i18n.tsx",
  fs.readFileSync("src/i18n.tsx", "utf8"),
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TSX,
);
function add(node) {
  if (
    ts.isPropertyAssignment(node) &&
    (ts.isStringLiteral(node.name) || ts.isIdentifier(node.name))
  )
    dictionaries[node.name.text] = node.initializer.text;
  ts.forEachChild(node, add);
}
add(source);
const missing = new Set(),
  invalid = [];
for (const file of fs
  .readdirSync("src")
  .filter((f) => f.endsWith(".tsx") && f !== "i18n.tsx")) {
  const code = ts.createSourceFile(
    file,
    fs.readFileSync("src/" + file, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TSX,
  );
  function visit(node) {
    if (
      ts.isCallExpression(node) &&
      node.expression.getText(code) === "tr" &&
      ts.isStringLiteral(node.arguments[0])
    ) {
      const key = node.arguments[0].text;
      if (
        !dictionaries[key] &&
        !["Focus", "Tags", "Scan", "Terminal"].includes(key)
      )
        missing.add(key);
      if (/[—–’:]/.test(key)) invalid.push({ file, key });
    }
    ts.forEachChild(node, visit);
  }
  visit(code);
}
console.log(JSON.stringify({ missing: [...missing], invalid }, null, 2));
if (missing.size || invalid.length) process.exitCode = 1;
