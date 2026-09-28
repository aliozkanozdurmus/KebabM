/** One gate per window. Terminal events remain visible to every subscriber. */
export class RequestGate {
  current?: string;
  private completed = new Set<string>();
  private cancelled = new Set<string>();
  private remember(set: Set<string>, id: string) {
    set.add(id);
    if (set.size > 200) set.delete(set.values().next().value!);
  }
  begin(id: string) { if (this.current && this.current !== id) this.cancel(); this.current = id; }
  start(id?: string) {
    if (!id) return !this.current;
    if (this.cancelled.has(id) || this.completed.has(id) || (this.current && this.current !== id)) return false;
    this.current = id;
    return true;
  }
  token(id?: string) { return id ? this.current === id && !this.cancelled.has(id) : !this.current; }
  end(id?: string) {
    if (!id) return !this.current;
    if (this.cancelled.has(id)) return false;
    if (this.completed.has(id)) return !this.current;
    if (this.current !== id) return false;
    this.remember(this.completed, id);
    this.current = undefined;
    return true;
  }
  cancel() {
    if (this.current) this.remember(this.cancelled, this.current);
    this.current = undefined;
  }
}
