export const UNWRITTEN_WORK_ITEM = "0002-second-work-item";

const UNWRITTEN_WORK_ITEM_CLUSTER = "second-work-item";

export const UNWRITTEN_WORK_ITEM_CARD = `li[data-relpath="meta/work/${UNWRITTEN_WORK_ITEM}.md"]`;

export const UNWRITTEN_LIFECYCLE_CARD = `li:has(> a[href="/lifecycle/${UNWRITTEN_WORK_ITEM_CLUSTER}"])`;
