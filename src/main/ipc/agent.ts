import type { VelaBackend } from "../opencode";
import type { FormAnswers, PermissionReply, PromptDelivery, PromptFile, WorkspaceIdentity } from "@shared/types";
import { formatFailure, normalizeFailure } from "@shared/errors";
import { workspaceId } from "../workspace-security";
import { commandPayload, optionalSelectionId, permissionPayload, providerCredentialPayload, promptPayload, queryText, selectionId, sessionId } from "../ipc-schema";
import type { TrustedIpcRegistrar } from "./registration";

export function registerAgentIpc(handleTrusted: TrustedIpcRegistrar, backend: VelaBackend): void {

  handleTrusted("shell:session-transcript", async (_e, sessionID: string) =>
    backend.sessionTranscript(sessionId(sessionID))
  );

  handleTrusted("shell:session-usage", async (_e, sessionID: string) =>
    backend.sessionUsage(sessionId(sessionID))
  );

  handleTrusted("shell:prompt", async (_e, workspace: WorkspaceIdentity, text: string, files: PromptFile[] = [], delivery?: PromptDelivery) => {
    try {
      const payload = promptPayload(workspace, text, files);
      return await backend.prompt(payload.workspace, payload.text, payload.files, delivery);
    } catch (error) {
      const failure = normalizeFailure(error, "VELA_PROMPT_FAILED", "Prompt failed");
      const reported = new Error(formatFailure(failure));
      reported.name = failure.code;
      Object.assign(reported, failure);
      throw reported;
    }
  });

  handleTrusted("shell:inbox-list", async (_e, workspace: WorkspaceIdentity) => backend.listInbox(workspace));

  handleTrusted("shell:inbox-cancel", async (_e, workspace: WorkspaceIdentity, inboxID: string) => backend.cancelInbox(workspace, inboxID));

  handleTrusted("shell:inbox-steer", async (_e, workspace: WorkspaceIdentity, inboxID: string) => backend.steerInbox(workspace, inboxID));

  handleTrusted("shell:forms-list", async (_e, workspace: WorkspaceIdentity) => backend.listForms(workspace));

  handleTrusted("shell:form-reply", async (_e, workspace: WorkspaceIdentity, formID: string, answers: FormAnswers, formSessionID?: string) => backend.replyForm(workspace, formID, answers, formSessionID));

  handleTrusted("shell:form-cancel", async (_e, workspace: WorkspaceIdentity, formID: string, formSessionID?: string) => backend.cancelForm(workspace, formID, formSessionID));

  handleTrusted("shell:commands", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listCommands(workspace);
  });

  handleTrusted("shell:run-command", async (_e, workspace: WorkspaceIdentity, name: string, args: string = "") => {
    await backend.workspaceDirectory(workspace);
    const command = commandPayload(name, args);
    return backend.runCommand(workspace, command.name, command.args);
  });

  handleTrusted("shell:find-files", async (_e, workspace: WorkspaceIdentity, query: string) => {
    workspaceId(workspace);
    return backend.searchFiles(workspace, queryText(query));
  });

  handleTrusted("shell:interrupt", async (_e, workspace: WorkspaceIdentity) => backend.interrupt(workspace));

  handleTrusted("shell:models", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listModels(workspace);
  });

  handleTrusted("shell:model-default", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.modelDefault(workspace);
  });

  handleTrusted("shell:switch-model", async (_e, workspace: WorkspaceIdentity, id: string, providerID: string, variant?: string) =>
    backend.switchModel(
      workspace,
      selectionId(id, "model id"),
      selectionId(providerID, "provider id"),
      optionalSelectionId(variant, "model variant")
    )
  );

  handleTrusted("shell:permission-reply", async (
    _e,
    workspace: WorkspaceIdentity,
    requestID: string,
    reply: PermissionReply,
    sessionID: string
  ) => {
    const permission = permissionPayload(requestID, reply, sessionID);
    return backend.replyPermission(workspace, permission.requestID, permission.reply, permission.sessionID);
  });

  handleTrusted("shell:list-permissions", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listPermissions(workspace);
  });

  handleTrusted("shell:agents", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listAgents(workspace);
  });

  handleTrusted("shell:switch-agent", async (_e, workspace: WorkspaceIdentity, id: string) =>
    backend.switchAgent(workspace, selectionId(id, "agent id")));

  handleTrusted("shell:state", async () => backend.getState());

  handleTrusted("shell:session-selection", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.sessionSelection(workspace);
  });

  handleTrusted("shell:mcp-list", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listMcpServers(workspace);
  });

  handleTrusted("shell:plugins-list", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listPlugins(workspace);
  });

  handleTrusted("shell:skills-list", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listSkills(workspace);
  });

  handleTrusted("shell:provider-usage", async () => backend.providerUsage());

  handleTrusted("shell:provider-integrations", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.listProviderIntegrations(workspace);
  });

  handleTrusted("shell:provider-key-connect", async (
    _e,
    workspace: WorkspaceIdentity,
    integrationID: unknown,
    key: unknown,
    label: unknown,
    answers: unknown
  ) => {
    workspaceId(workspace);
    const credential = providerCredentialPayload(integrationID, key, label, answers);
    return backend.connectProviderKey(workspace, credential.integrationID, credential.key, credential.label, credential.answers);
  });

  handleTrusted("shell:provider-oauth-start", async (_e, workspace: WorkspaceIdentity, integrationID: string, methodID: string) => {
    workspaceId(workspace);
    return backend.startProviderOAuth(workspace, integrationID, methodID);
  });

  handleTrusted("shell:provider-oauth-poll", async (_e, workspace: WorkspaceIdentity, integrationID: string, attemptID: string) => {
    workspaceId(workspace);
    return backend.pollProviderOAuth(workspace, integrationID, attemptID);
  });

  handleTrusted("shell:provider-oauth-complete", async (_e, workspace: WorkspaceIdentity, integrationID: string, attemptID: string, code?: string) => {
    workspaceId(workspace);
    return backend.completeProviderOAuth(workspace, integrationID, attemptID, code);
  });

  handleTrusted("shell:provider-oauth-cancel", async (_e, workspace: WorkspaceIdentity, integrationID: string, attemptID: string) => {
    workspaceId(workspace);
    return backend.cancelProviderOAuth(workspace, integrationID, attemptID);
  });

  handleTrusted("shell:session-revert-stage", async (_e, workspace: WorkspaceIdentity, messageID: string, files: boolean) => {
    workspaceId(workspace);
    return backend.stageRevert(workspace, messageID, files);
  });

  handleTrusted("shell:session-revert-commit", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.commitRevert(workspace);
  });

  handleTrusted("shell:session-revert-clear", async (_e, workspace: WorkspaceIdentity) => {
    workspaceId(workspace);
    return backend.clearRevert(workspace);
  });

  handleTrusted("shell:provider-credential-remove", async (_e, workspace: WorkspaceIdentity, credentialID: unknown) => {
    workspaceId(workspace);
    return backend.removeProviderCredential(workspace, selectionId(credentialID, "provider credential id"));
  });
}
