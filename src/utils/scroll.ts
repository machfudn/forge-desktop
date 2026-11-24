/**
 * Svelte action to allow standard vertical mouse wheel scrolling (deltaY)
 * to smoothly scroll a horizontal-only overflow container.
 * 
 * Preserves native horizontal trackpad gestures (when deltaX is dominant or Shift is held).
 */
export function wheelHorizontalScroll(node: HTMLElement) {
  function handleWheel(e: WheelEvent) {
    // If Shift is held or there's no vertical delta, allow native handling
    if (e.deltaY === 0 || e.shiftKey) return;

    // If user is performing a native horizontal swipe on a touchpad, do not interfere
    if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;

    // Check if the container actually has horizontal overflow
    const maxScrollLeft = node.scrollWidth - node.clientWidth;
    if (maxScrollLeft <= 0) return;

    // Check if we can scroll in the requested direction
    const isScrollingRight = e.deltaY > 0;
    const isScrollingLeft = e.deltaY < 0;

    const canScrollRight = node.scrollLeft < maxScrollLeft - 1;
    const canScrollLeft = node.scrollLeft > 1;

    if ((isScrollingRight && canScrollRight) || (isScrollingLeft && canScrollLeft)) {
      node.scrollLeft += e.deltaY;
      e.preventDefault();
    }
  }

  node.addEventListener("wheel", handleWheel, { passive: false });

  return {
    destroy() {
      node.removeEventListener("wheel", handleWheel);
    },
  };
}
