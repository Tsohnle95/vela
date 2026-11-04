export class InspectPickerState {
  private token = 0;
  private activeValue = false;

  get active(): boolean {
    return this.activeValue;
  }

  begin(): number {
    this.activeValue = true;
    return ++this.token;
  }

  isCurrent(token: number): boolean {
    return this.activeValue && token === this.token;
  }

  cancel(): void {
    this.activeValue = false;
    this.token++;
  }

  claim(): boolean {
    if (!this.activeValue) return false;
    this.cancel();
    return true;
  }
}
