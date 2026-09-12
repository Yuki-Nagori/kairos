/**
 * localStorage 统一网关：所有持久化 key 经 storageKey 生成（全局唯一前缀
 * `kairos:<域>:<名>`），值一律 JSON 序列化——布尔/对象/数组同形，损坏数据
 * 安全回退缺省。此前 4 处调用点各写裸字符串 key 且值格式不一（"1"/"0" 与
 * JSON 混用），工艺预设靠遍历 localStorage.length 扫 key；统一后键控集合
 * 用索引 key 管理，增删都不扫描。
 */

export function storageKey(domain: string, name: string): string {
  return `kairos:${domain}:${name}`;
}

/** 读取 JSON 值；缺 key 或数据损坏（手改 / 旧版本残留）回退 fallback。 */
export function storageGet<T>(key: string, fallback: T): T {
  const raw = localStorage.getItem(key);
  if (raw === null) {
    return fallback;
  }
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function storageSet(key: string, value: unknown): void {
  localStorage.setItem(key, JSON.stringify(value));
}

export function storageRemove(key: string): void {
  localStorage.removeItem(key);
}

/** 键控集合：名字清单存单个索引 key，条目各占一个 key（列表不扫描全量 key）。 */
interface StorageIndex {
  list(domain: string): string[];
  /** 登记名字（幂等；条目值由调用方经 storageSet 独立写入）。 */
  add(domain: string, name: string): void;
  /** 注销名字并删除其条目。 */
  remove(domain: string, name: string): void;
}

export function storageIndex(): StorageIndex {
  const indexKey = (domain: string) => storageKey(domain, "index");
  const read = (domain: string): string[] => storageGet(indexKey(domain), [] as string[]);
  return {
    list: (domain) => read(domain),
    add(domain, name) {
      const names = read(domain);
      if (!names.includes(name)) {
        storageSet(indexKey(domain), [...names, name]);
      }
    },
    remove(domain, name) {
      storageSet(
        indexKey(domain),
        read(domain).filter((existing) => existing !== name),
      );
      storageRemove(storageKey(domain, name));
    },
  };
}
