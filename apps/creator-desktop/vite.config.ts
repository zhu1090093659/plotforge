import react from "@vitejs/plugin-react";
import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vitest/config";

const tauriDevHost = process.env.TAURI_DEV_HOST;
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

export default defineConfig({
  plugins: [react(), plotforgeStudioApi()],
  clearScreen: false,
  server: {
    host: tauriDevHost ?? "127.0.0.1",
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});

function plotforgeStudioApi(): Plugin {
  return {
    name: "plotforge-studio-api",
    configureServer(server) {
      server.middlewares.use("/__plotforge_studio/pick-directory", async (request, response) => {
        if (request.method !== "POST") {
          response.statusCode = 405;
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify({ code: "method_not_allowed", message: "POST required" }));
          return;
        }

        try {
          const selected = await pickProjectDirectory();
          response.statusCode = 200;
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify(selected));
        } catch (source) {
          response.statusCode = 500;
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify({
            code: "directory_picker",
            message: source instanceof Error ? source.message : String(source),
          }));
        }
      });

      server.middlewares.use("/__plotforge_studio/invoke", async (request, response) => {
        if (request.method !== "POST") {
          response.statusCode = 405;
          response.setHeader("content-type", "application/json");
          response.end(JSON.stringify({ code: "method_not_allowed", message: "POST required" }));
          return;
        }

        try {
          const payload = JSON.parse(await readRequestBody(request)) as {
            command?: unknown;
            args?: unknown;
          };
          if (typeof payload.command !== "string" || !payload.command) {
            throw new Error("command is required");
          }

          const result = await runStudioCommand(
            payload.command,
            isRecord(payload.args) ? payload.args : {},
          );
          response.statusCode = 200;
          response.setHeader("content-type", "application/json");
          response.end(result);
        } catch (source) {
          response.statusCode = 500;
          response.setHeader("content-type", "application/json");
          response.end(
            JSON.stringify({
              code: "studio_dev_bridge",
              message: source instanceof Error ? source.message : String(source),
            }),
          );
        }
      });
    },
  };
}

function pickProjectDirectory(): Promise<string | null> {
  const command = directoryPickerCommand();
  return new Promise((resolveDirectory, reject) => {
    const child = spawn(command.program, command.args, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolveDirectory(stdout.trim() || null);
        return;
      }
      if (isDirectoryPickerCancel(code, stderr)) {
        resolveDirectory(null);
        return;
      }
      reject(new Error(stderr.trim() || `Directory picker exited with code ${code}`));
    });
  });
}

function directoryPickerCommand(): { program: string; args: string[] } {
  if (process.platform === "darwin") {
    return {
      program: "osascript",
      args: [
        "-e",
        'POSIX path of (choose folder with prompt "Open PlotForge project folder")',
      ],
    };
  }
  if (process.platform === "win32") {
    return {
      program: "powershell.exe",
      args: [
        "-NoProfile",
        "-Command",
        "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description = 'Open PlotForge project folder'; if ($d.ShowDialog() -eq 'OK') { Write-Output $d.SelectedPath }",
      ],
    };
  }
  return {
    program: "zenity",
    args: ["--file-selection", "--directory", "--title=Open PlotForge project folder"],
  };
}

function isDirectoryPickerCancel(code: number | null, stderr: string) {
  if (process.platform === "darwin") {
    return stderr.includes("User canceled") || stderr.includes("(-128)");
  }
  return code === 1;
}

function readRequestBody(request: NodeJS.ReadableStream) {
  return new Promise<string>((resolveBody, reject) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk: string) => {
      body += chunk;
    });
    request.on("end", () => resolveBody(body));
    request.on("error", reject);
  });
}

function runStudioCommand(command: string, args: Record<string, unknown>) {
  return new Promise<string>((resolveCommand, reject) => {
    const child = spawn(
      "cargo",
      ["run", "--quiet", "-p", "plotforge-cli", "--", "studio", command],
      {
        cwd: repoRoot,
        stdio: ["pipe", "pipe", "pipe"],
      },
    );
    let stdout = "";
    let stderr = "";
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error(`Studio command ${command} timed out`));
    }, 120_000);

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });
    child.on("error", (source) => {
      clearTimeout(timeout);
      reject(source);
    });
    child.on("close", (code) => {
      clearTimeout(timeout);
      if (code === 0) {
        resolveCommand(stdout.trim() || "null");
        return;
      }
      reject(new Error(stderr.trim() || `Studio command ${command} failed with exit ${code}`));
    });
    child.stdin.end(JSON.stringify(args));
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
