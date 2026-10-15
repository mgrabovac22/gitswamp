import type { HttpResponseData, HttpTestResult } from "./httpClient.types";

export interface ScriptEnvironment {
  values: Record<string, string>;
}

export interface TestContext {
  response: HttpResponseData;
  variables: Record<string, string>;
}

export interface PmApi {
  test: (name: string, fn: () => void) => void;
  expect: (actual: unknown) => PmExpectation;
  environment: {
    set: (name: string, value: unknown) => void;
    get: (name: string) => string | undefined;
    unset: (name: string) => void;
  };
  variables: {
    set: (name: string, value: unknown) => void;
    get: (name: string) => string | undefined;
  };
  response: {
    code: number;
    status: string;
    responseTime: number;
    responseSize: number;
    json: () => unknown;
    text: () => string;
    headers: Record<string, string>;
  };
}

export interface PmExpectation {
  toEql: (expected: unknown) => void;
  toBe: (expected: unknown) => void;
  toBeTruthy: () => void;
  toBeFalsy: () => void;
  toBeGreaterThan: (value: number) => void;
  toBeLessThan: (value: number) => void;
  toInclude: (value: unknown) => void;
}

function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  try {
    return JSON.stringify(a) === JSON.stringify(b);
  } catch {
    return false;
  }
}

function createExpectation(actual: unknown): PmExpectation {
  return {
    toEql: (expected) => {
      if (!deepEqual(actual, expected)) {
        throw new Error(`Expected ${JSON.stringify(expected)} but received ${JSON.stringify(actual)}`);
      }
    },
    toBe: (expected) => {
      if (actual !== expected) {
        throw new Error(`Expected ${String(expected)} but received ${String(actual)}`);
      }
    },
    toBeTruthy: () => {
      if (!actual) throw new Error(`Expected value to be truthy but received ${String(actual)}`);
    },
    toBeFalsy: () => {
      if (actual) throw new Error(`Expected value to be falsy but received ${String(actual)}`);
    },
    toBeGreaterThan: (value) => {
      if (typeof actual !== "number" || actual <= value) {
        throw new Error(`Expected ${String(actual)} to be greater than ${value}`);
      }
    },
    toBeLessThan: (value) => {
      if (typeof actual !== "number" || actual >= value) {
        throw new Error(`Expected ${String(actual)} to be less than ${value}`);
      }
    },
    toInclude: (value) => {
      if (typeof actual === "string" && typeof value === "string") {
        if (!actual.includes(value)) {
          throw new Error(`Expected "${actual}" to include "${value}"`);
        }
        return;
      }
      if (Array.isArray(actual)) {
        if (!actual.some((item) => deepEqual(item, value))) {
          throw new Error(`Expected array to include ${JSON.stringify(value)}`);
        }
        return;
      }
      throw new Error("toInclude expects a string or array value.");
    },
  };
}

function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

export function runPreRequestScript(script: string, variables: Record<string, string>): { ran: boolean; error: string | null } {
  const trimmed = script.trim();
  if (!trimmed) return { ran: false, error: null };

  const pm = {
    environment: {
      set: (name: string, value: unknown) => {
        variables[name] = value === undefined ? "" : String(value);
      },
      get: (name: string) => variables[name],
      unset: (name: string) => {
        delete variables[name];
      },
    },
    variables: {
      set: (name: string, value: unknown) => {
        variables[name] = value === undefined ? "" : String(value);
      },
      get: (name: string) => variables[name],
    },
  };

  try {
    const executor = new Function("pm", "console", `"use strict";\n${trimmed}`);
    executor(pm, { log: () => {}, warn: () => {}, error: () => {} });
    return { ran: true, error: null };
  } catch (error) {
    return { ran: true, error: messageOf(error) };
  }
}

export function runTests(script: string, context: TestContext): HttpTestResult[] {
  const trimmed = script.trim();
  if (!trimmed) return [];

  const results: HttpTestResult[] = [];
  const headerMap: Record<string, string> = {};
  for (const header of context.response.headers) {
    const key = header.name.toLowerCase();
    headerMap[key] = headerMap[key] ? `${headerMap[key]}, ${header.value}` : header.value;
  }

  const pm: PmApi = {
    test: (name, fn) => {
      try {
        fn();
        results.push({ name, passed: true, message: "" });
      } catch (error) {
        results.push({ name, passed: false, message: messageOf(error) });
      }
    },
    expect: createExpectation,
    environment: {
      set: (name, value) => {
        context.variables[name] = value === undefined ? "" : String(value);
      },
      get: (name) => context.variables[name],
      unset: (name) => {
        delete context.variables[name];
      },
    },
    variables: {
      set: (name, value) => {
        context.variables[name] = value === undefined ? "" : String(value);
      },
      get: (name) => context.variables[name],
    },
    response: {
      code: context.response.status,
      status: context.response.statusText,
      responseTime: context.response.durationMs,
      responseSize: context.response.sizeBytes,
      headers: headerMap,
      text: () => context.response.body,
      json: () => {
        try {
          return JSON.parse(context.response.body);
        } catch {
          throw new Error("Response body is not valid JSON.");
        }
      },
    },
  };

  try {
    const executor = new Function("pm", "console", `"use strict";\n${trimmed}`);
    executor(pm, { log: () => {}, warn: () => {}, error: () => {} });
  } catch (error) {
    results.push({ name: "Script error", passed: false, message: messageOf(error) });
  }

  return results;
}

export const TEST_SCRIPT_HINT = `pm.test("status is 200", () => {
  pm.expect(pm.response.code).toBe(200);
});

pm.test("has expected field", () => {
  const data = pm.response.json();
  pm.expect(data).toInclude ? true : true;
});`;

export const PRE_REQUEST_SCRIPT_HINT = `// Runs before the request is sent.
// pm.environment.set("token", "value");
// Variables set here can be used as {{token}}.`;
