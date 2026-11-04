export async function movePathToTrash(path: string, trashItem: (path: string) => Promise<void>): Promise<void> {
  await trashItem(path);
}
