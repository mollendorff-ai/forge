// Read an .xlsx file (a zip archive) into its parts, with Node's own zlib,
// and compare two workbooks part by part. A workbook records when it was
// made, in docProps/core.xml, so two exports of the same model are never
// equal byte for byte; everything else must be.

import { inflateRawSync } from "node:zlib";

export function parts(bytes) {
  const b = Buffer.from(bytes);
  let end = b.length - 22;
  while (end >= 0 && b.readUInt32LE(end) !== 0x06054b50) end--;
  if (end < 0) throw new Error("not a zip archive: no end of central directory");
  const count = b.readUInt16LE(end + 10);
  let at = b.readUInt32LE(end + 16);
  const out = new Map();
  for (let i = 0; i < count; i++) {
    if (b.readUInt32LE(at) !== 0x02014b50) throw new Error("bad central directory");
    const method = b.readUInt16LE(at + 10), size = b.readUInt32LE(at + 20);
    const nameLength = b.readUInt16LE(at + 28), extra = b.readUInt16LE(at + 30), comment = b.readUInt16LE(at + 32);
    const local = b.readUInt32LE(at + 42), name = b.toString("utf8", at + 46, at + 46 + nameLength);
    const start = local + 30 + b.readUInt16LE(local + 26) + b.readUInt16LE(local + 28);
    const data = b.subarray(start, start + size);
    out.set(name, method === 0 ? Buffer.from(data) : method === 8 ? inflateRawSync(data) : null);
    at += 46 + nameLength + extra + comment;
  }
  return out;
}

const CREATED = /<dcterms:(created|modified)[^>]*>[^<]*<\/dcterms:\1>/g;

// The differences between two workbooks, as sentences; [] when they hold the
// same parts, in the same order, with the same content, apart from when each
// was made.
export function differences(a, b) {
  const pa = parts(a), pb = parts(b), found = [];
  const na = [...pa.keys()], nb = [...pb.keys()];
  if (na.join("\n") !== nb.join("\n")) found.push(`the parts differ: ${na.join(", ")} against ${nb.join(", ")}`);
  for (const name of na) {
    if (!pb.has(name)) continue;
    let x = pa.get(name), y = pb.get(name);
    if (name === "docProps/core.xml") { x = Buffer.from(x.toString("utf8").replace(CREATED, "")); y = Buffer.from(y.toString("utf8").replace(CREATED, "")); }
    if (!x || !y || !x.equals(y)) found.push(`${name} differs`);
  }
  return found;
}
