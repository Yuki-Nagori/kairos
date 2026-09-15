/**
 * 可折叠卡片的状态：折叠偏好按面板标题持久化（跨会话保持），
 * 不可折叠时整条路径都不写存储。
 */
import { ref } from "vue";
import { storageGet, storageKey, storageSet } from "../../utils/storage";

export function useCollapsibleCard(props: { title: string; collapsible: boolean }) {
  const collapsed = ref(
    props.collapsible ? storageGet(storageKey("panel", props.title), false) : false,
  );

  function toggle(): void {
    if (!props.collapsible) {
      return;
    }
    collapsed.value = !collapsed.value;
    storageSet(storageKey("panel", props.title), collapsed.value);
  }

  return { collapsed, toggle };
}
