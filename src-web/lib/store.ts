type StoreListener<T> = (state: T) => void;

interface Store<T> {
  get(): T;
  /** 浅合并 patch 并全量通知订阅者。 */
  set(patch: Partial<T>): void;
  /** 全量订阅：每次 set 都会通知（无论是否涉及相关切片）。返回取消订阅函数。 */
  subscribe(listener: StoreListener<T>): () => void;
}

/**
 * 极简发布订阅状态容器：唯一可变状态源，`set` 浅合并 patch。
 * 组件按需选择 `subscribe`（关心多个切片）或 `select`（只关心单个切片）。
 */
export function createStore<T extends object>(initial: T): Store<T> {
  let state = initial;
  const listeners = new Set<StoreListener<T>>();
  return {
    get: () => state,
    set: (patch) => {
      state = { ...state, ...patch };
      for (const listener of listeners) {
        listener(state);
      }
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/**
 * 选择器订阅：只关心状态切片的组件用它避免无关更新。
 * 监听器立即以当前值调用一次；之后仅当选择值（Object.is 比较）变化时触发。
 */
export function select<T, S>(
  store: Store<T>,
  selector: (state: T) => S,
  listener: (value: S) => void,
): () => void {
  let current = selector(store.get());
  listener(current);
  return store.subscribe((state) => {
    const next = selector(state);
    if (!Object.is(next, current)) {
      current = next;
      listener(next);
    }
  });
}
