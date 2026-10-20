import type { SessionInfo } from "@shared/types";
export const demoSession: SessionInfo = {id:"comparison", title:"Renderer comparison", directory:"/comparison", runtimeID:"opencode", workspace:{id:"11111111-1111-4111-8111-111111111111",generation:1}};
export function installComparisonBridge() {
  const empty = async () => [];
  window.vela = {
    platform:"darwin",onMessage:() => () => {},health:async () => true,state:async () => null,
    takePendingPaths:empty,activeSessions:empty,sessions:empty,models:empty,agents:empty,
    modelDefault:async () => null,sessionSelection:async () => null,savedWorkspaces:async () => ({initialized:true,workspaces:[]}),saveSavedWorkspaces:async () => {},
    formsList:empty,inboxList:empty,listPermissions:empty,recoveryRecords:empty,closeSession:async () => {},
    openSession:async () => demoSession, openSessionById:async () => ({session:demoSession,transcript:[],todos:[],usage:null}),
    sessionTranscript:async () => ({transcript:[],todos:[],usage:null}),sessionUsage:async () => null,
    listDir:empty,runtimes:empty,projects:empty,providerUsage:async () => null,
    readFile:async () => "",reportUnsavedEditorFiles:() => {},permissionReply:async () => {},
  } as unknown as typeof window.vela;
}
