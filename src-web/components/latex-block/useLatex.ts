/**
 * LaTeX 渲染逻辑（KaTeX）：render 直接构建 DOM 节点挂进容器，
 * 不经 HTML 字符串解析，从结构上消除注入面；个别公式错误经
 * throwOnError 关闭不炸 UI。颜色经 currentColor 随主题。
 * 容器引用由组件声明后传入（静态模板 ref 不被 vue-tsc 计为读取），
 * tex / displayMode 以 getter 传入，保持对 props 的响应式追踪。
 */
import { onMounted, watch, type Ref } from "vue";
import katex from "katex";

export function useLatex(
  container: Ref<HTMLDivElement | null>,
  tex: () => string,
  displayMode: () => boolean,
): void {
  function render(): void {
    if (container.value !== null) {
      katex.render(tex(), container.value, {
        displayMode: displayMode(),
        throwOnError: false,
      });
    }
  }

  onMounted(render);
  watch([tex, displayMode], render);
}
