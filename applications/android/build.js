import { exec } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";
import { cp, mkdir } from "node:fs/promises";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Helper function to execute shell commands with proper error handling
// Handles both Windows and Unix-like systems
function command(cmd, options = {}, output = true) {
    return new Promise(
        (
            resolve,
            reject,
            ps = exec(
                process.platform == "win32"
                    ? "$ProgressPreference = 'SilentlyContinue';" + cmd
                    : cmd,
                {
                    shell: process.platform == "win32" ? "powershell.exe" : "bash",
                    env: process.env,
                    stdio: "inherit",
                    cwd: __dirname,
                    ...options,
                }
            )
        ) => {
            if (output) {
                ps.stdout.pipe(process.stdout);
                ps.stderr.pipe(process.stderr);
            }

            ps.on("error", reject);
            ps.on("close", (code) => {
                code == 0 ? resolve() : reject(code || 0);
            });
        }
    );
}

(async () => {
    await command("./gradlew assembleRelease");

    if (!existsSync("../../target/app")) {
        await mkdir("../../target/app");
    }

    await cp("app/build/outputs/apk/release/app-release.apk", "../../target/app/Hylarana.apk");
})();
