export function protectEditorUnload(event: BeforeUnloadEvent, unsavedFileCount: number): void {
  if (unsavedFileCount <= 0) return;
  event.preventDefault();
  event.returnValue = "";
}
