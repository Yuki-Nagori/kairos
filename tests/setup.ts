/**
 * 测试环境保真度修补：happy-dom 未实现 document.compatMode（读到 undefined），
 * KaTeX 在模块加载时检测到非标准模式会把 render() 永久替换成抛错桩。
 * 真实应用 index.html 带 doctype、恒为标准模式，这里补齐该字段使测试一致。
 */
Object.defineProperty(document, "compatMode", { value: "CSS1Compat" });
