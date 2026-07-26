// targets/pi/extension.ts
// Dev-flow lifecycle hooks for Pi agent — dow CLI unified dispatch
//
// Internal Framework:
// extension.ts
// └── default export (ExtensionAPI)
//     ├── input event         → dow hooks context -H (context injection)
//     ├── tool_call event     → dow hooks guard (write guardian)
//     ├── tool_result event   → dow hooks post-write / post-bash
//     └── session_shutdown    → dow hooks session-stop (revoke claims + save changelog)
//
// Related Docs:
// - [CLAUDE.md - Hooks](../../CLAUDE.md#hooks)

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function devFlowExtension(pi: ExtensionAPI) {
	// ─── Context injection (equivalent to UserPromptSubmit) ───────────────────────

	pi.on("input", async (_event, ctx) => {
		const result = await pi.exec("dow", ["hooks", "context", "-H"]);
		if (result.code === 0 && result.stdout.trim()) {
			ctx.ui.notify(result.stdout.trim(), "info");
		}
		return { action: "continue" as const };
	});

	// ─── Write guardian (equivalent to PreToolUse Write|Edit|Bash) ────────────────

	pi.on("tool_call", async (event, ctx) => {
		if (event.toolName === "write" || event.toolName === "edit") {
			const filePath =
				"input" in event && event.input
					? (event.input as Record<string, unknown>).file_path ||
						(event.input as Record<string, unknown>).path ||
						""
					: "";
			const result = await pi.exec("dow", [
				"hooks",
				"guard",
				String(filePath),
			]);
			if (result.code !== 0 || isBlocked(result.stdout)) {
				const reason = extractDenyReason(result.stdout) || result.stderr.trim();
				return { block: true, reason };
			}
		} else if (event.toolName === "bash") {
			const command =
				"input" in event && event.input
					? (event.input as Record<string, unknown>).command || ""
					: "";
			const guardInput = JSON.stringify({
				tool_name: "Bash",
				tool_input: { command: String(command) },
			});
			const result = await pi.exec("dow", ["hooks", "guard"], {
				stdin: guardInput,
			});
			if (result.code !== 0 || isBlocked(result.stdout)) {
				const reason = extractDenyReason(result.stdout) || result.stderr.trim();
				return { block: true, reason };
			}
		}
		return undefined;
	});

	// ─── Post-write linkage (equivalent to PostToolUse Write|Edit) ────────────────

	pi.on("tool_result", async (event, _ctx) => {
		if (event.toolName === "write" || event.toolName === "edit") {
			const filePath =
				"input" in event && event.input
					? (event.input as Record<string, unknown>).file_path ||
						(event.input as Record<string, unknown>).path ||
						""
					: "";
			await pi.exec("dow", ["hooks", "post-write", String(filePath)]);
		} else if (event.toolName === "bash") {
			const command =
				"input" in event && event.input
					? (event.input as Record<string, unknown>).command || ""
					: "";
			await pi.exec("dow", ["hooks", "post-bash", String(command)]);
		}
		return undefined;
	});

	// ─── Unified session end handler (revoke claims + save changelog) ────────────

	pi.on("session_shutdown", async (_event, _ctx) => {
		await pi.exec("dow", ["hooks", "session-stop"]);
	});
}

function isBlocked(stdout: string): boolean {
	try {
		const parsed = JSON.parse(stdout);
		const decision =
			parsed?.hookSpecificOutput?.permissionDecision ||
			parsed?.decision;
		return decision === "deny" || decision === "block";
	} catch {
		return false;
	}
}

function extractDenyReason(stdout: string): string | undefined {
	try {
		const parsed = JSON.parse(stdout);
		return (
			parsed?.hookSpecificOutput?.permissionDecisionReason ||
			parsed?.reason ||
			undefined
		);
	} catch {
		return undefined;
	}
}
