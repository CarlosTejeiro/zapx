<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'

  type ResizeDir = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West'

  const win = getCurrentWindow()

  function onDown(dir: ResizeDir) {
    return (e: MouseEvent) => {
      e.preventDefault()
      win.startResizeDragging(dir)
    }
  }
</script>

<!-- 4 edges -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-n"  onmousedown={onDown('North')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-s"  onmousedown={onDown('South')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-e"  onmousedown={onDown('East')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-w"  onmousedown={onDown('West')}></div>

<!-- 4 corners -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-ne" onmousedown={onDown('NorthEast')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-nw" onmousedown={onDown('NorthWest')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-se" onmousedown={onDown('SouthEast')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rh rh-sw" onmousedown={onDown('SouthWest')}></div>

<style>
  .rh {
    position: fixed;
    z-index: 9999;
    background: transparent;
  }

  /* Edges */
  .rh-n  { top: 0;    left: 8px;  right: 8px;  height: 4px; cursor: n-resize; }
  .rh-s  { bottom: 0; left: 8px;  right: 8px;  height: 4px; cursor: s-resize; }
  .rh-e  { top: 8px;  bottom: 8px; right: 0;   width: 4px;  cursor: e-resize; }
  .rh-w  { top: 8px;  bottom: 8px; left: 0;    width: 4px;  cursor: w-resize; }

  /* Corners */
  .rh-ne { top: 0;    right: 0;   width: 8px;  height: 8px; cursor: ne-resize; }
  .rh-nw { top: 0;    left: 0;    width: 8px;  height: 8px; cursor: nw-resize; }
  .rh-se { bottom: 0; right: 0;   width: 8px;  height: 8px; cursor: se-resize; }
  .rh-sw { bottom: 0; left: 0;    width: 8px;  height: 8px; cursor: sw-resize; }
</style>
