#!/usr/bin/env node

require("ts-node/register/transpile-only");

const fs = require("fs");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..");
const specificationsPath = path.join(
  repoRoot,
  "src/table/generated/artifacts/specifications.ts"
);
const mappersPath = path.join(repoRoot, "src/table/generated/artifacts/mappers.ts");
const parametersPath = path.join(
  repoRoot,
  "src/table/generated/artifacts/parameters.ts"
);
const operationPath = path.join(repoRoot, "src/table/generated/artifacts/operation.ts");
const outputDirectory = path.join(
  repoRoot,
  "rust/crates/azurite-table/src/generated/artifacts/metadata"
);
const outputPath = path.join(outputDirectory, "specifications.generated.json");

const specificationsModule = require(specificationsPath);
const mappersModule = require(mappersPath);
const parametersModule = require(parametersPath);
const operationModule = require(operationPath);

const Specifications = specificationsModule.default || specificationsModule.Specifications;
const Operation = operationModule.Operation || operationModule.default;

if (!Specifications || typeof Specifications !== "object") {
  throw new Error("Unable to load table Specifications export.");
}

if (!Operation || typeof Operation !== "object") {
  throw new Error("Unable to load table Operation enum.");
}

if (!Object.keys(mappersModule).length) {
  throw new Error("Unable to load table mappers.");
}

if (!Object.keys(parametersModule).length) {
  throw new Error("Unable to load table parameters.");
}

function sanitize(value) {
  if (value === undefined || typeof value === "function") {
    return undefined;
  }

  if (Array.isArray(value)) {
    return value
      .map((item) => sanitize(item))
      .filter((item) => item !== undefined);
  }

  if (value && typeof value === "object") {
    const serialized = {};

    for (const [key, nestedValue] of Object.entries(value)) {
      if (key === "serializer") {
        continue;
      }

      const sanitizedValue = sanitize(nestedValue);
      if (sanitizedValue !== undefined) {
        serialized[key] = sanitizedValue;
      }
    }

    return serialized;
  }

  return value;
}

function serializeSpecification(operationName, operationIndex, specification) {
  if (!specification) {
    throw new Error(
      `Missing specification for operation ${operationName} (${operationIndex}).`
    );
  }

  return sanitize({
    operation: operationName,
    httpMethod: specification.httpMethod,
    path: specification.path,
    urlParameters: specification.urlParameters,
    queryParameters: specification.queryParameters,
    headerParameters: specification.headerParameters,
    requestBody: specification.requestBody,
    contentType: specification.contentType,
    responses: specification.responses,
    isXML: specification.isXML
  });
}

const orderedOperations = Object.entries(Operation)
  .filter(([, value]) => typeof value === "number")
  .sort(([, left], [, right]) => left - right);

const serializedSpecifications = orderedOperations.map(([operationName, operationIndex]) =>
  serializeSpecification(operationName, operationIndex, Specifications[operationIndex])
);

fs.mkdirSync(outputDirectory, { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(serializedSpecifications, null, 2)}\n`, "utf8");

JSON.parse(fs.readFileSync(outputPath, "utf8"));

console.log(
  `Generated ${serializedSpecifications.length} table specifications at ${outputPath}`
);
