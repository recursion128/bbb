use std::cell::Cell;

thread_local! {
    static STACK_EXTREMES: Cell<(usize, usize)> =
        const { Cell::new((0, usize::MAX)) };
}

pub(crate) fn reset(caller_marker: &u8) {
    let address = std::ptr::from_ref(caller_marker) as usize;
    STACK_EXTREMES.with(|cell| cell.set((address, address)));
}

pub(crate) fn record() {
    let marker = 0u8;
    let address = std::ptr::from_ref(&marker) as usize;
    STACK_EXTREMES.with(|cell| {
        let (top, bottom) = cell.get();
        cell.set((top.max(address), bottom.min(address)));
    });
}

pub(crate) fn used_bytes() -> usize {
    STACK_EXTREMES.with(|cell| {
        let (top, bottom) = cell.get();
        top.saturating_sub(bottom)
    })
}
