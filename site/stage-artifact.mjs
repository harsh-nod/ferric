import { copyFile, lstat, mkdir, readdir, rm } from "node:fs/promises";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const siteRoot = dirname(fileURLToPath(import.meta.url));
const outputRoot = process.argv[2] ? resolve(process.argv[2]) : null;
const deployableFiles = [
  "app.js",
  "assets/architecture.svg",
  "assets/mark.svg",
  "data/project.js",
  "index.html",
  "styles.css",
].sort();
const maximumArtifactBytes = 2 * 1024 * 1024;

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function filesBelow(root, directory = root) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await filesBelow(root, path)));
    } else {
      assert(entry.isFile(), `artifact contains a non-file entry: ${relative(root, path)}`);
      files.push(relative(root, path).split(sep).join("/"));
    }
  }
  return files;
}

assert(outputRoot, "usage: node stage-artifact.mjs OUTPUT_DIRECTORY");
const relativeOutput = relative(siteRoot, outputRoot);
assert(
  (relativeOutput === ".." || relativeOutput.startsWith(`..${sep}`)) &&
    outputRoot.split(sep).at(-1).startsWith("ferric-pages-artifact"),
  "artifact output must be a staging directory outside the site source",
);

await rm(outputRoot, { recursive: true, force: true });
for (const file of deployableFiles) {
  const source = join(siteRoot, file);
  const sourceStatus = await lstat(source);
  assert(sourceStatus.isFile() && !sourceStatus.isSymbolicLink(), `${file} must be a regular file`);
  const destination = join(outputRoot, file);
  await mkdir(dirname(destination), { recursive: true });
  await copyFile(source, destination);
}

const artifactFiles = (await filesBelow(outputRoot)).sort();
assert(
  JSON.stringify(artifactFiles) === JSON.stringify(deployableFiles),
  `artifact roster drifted: ${artifactFiles.join(", ")}`,
);
assert(
  artifactFiles.every(
    (file) =>
      !file.includes("node_modules") &&
      !/(?:^|\/)(?:package(?:-lock)?\.json|README\.md|.*(?:test|validate).*\.mjs)$/.test(file),
  ),
  "artifact contains dependency metadata or test-only files",
);

let totalBytes = 0;
for (const file of artifactFiles) {
  totalBytes += (await lstat(join(outputRoot, file))).size;
}
assert(totalBytes <= maximumArtifactBytes, `artifact is unexpectedly large: ${totalBytes} bytes`);
console.log(`Staged ${artifactFiles.length} static files (${totalBytes} bytes): ${artifactFiles.join(", ")}`);
