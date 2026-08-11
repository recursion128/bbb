#import <QuartzCore/CAMetalLayer.h>
#import <objc/runtime.h>
#include <stdio.h>

// Capture-only compatibility probe for the fixed wgpu 0.19 baseline binary.
// The current client logs these values natively. The old immutable binary does
// not, so this interposer observes its first drawable acquisition, restores the
// original implementation immediately, and leaves the measured frame loop
// untouched.
static Method bbb_next_drawable_method;
static IMP bbb_next_drawable_implementation;

static id bbb_observe_next_drawable(CAMetalLayer *layer, SEL selector) {
    method_setImplementation(
        bbb_next_drawable_method, bbb_next_drawable_implementation
    );
    CGSize size = layer.drawableSize;
    fprintf(
        stderr,
        "external Metal layer presentation observed drawable_width=%.17g "
        "drawable_height=%.17g maximum_drawable_count=%lu\n",
        (double)size.width,
        (double)size.height,
        (unsigned long)layer.maximumDrawableCount
    );
    fflush(stderr);
    return ((id (*)(id, SEL))bbb_next_drawable_implementation)(layer, selector);
}

__attribute__((constructor)) static void bbb_install_metal_layer_probe(void) {
    Class metal_layer = objc_getClass("CAMetalLayer");
    if (metal_layer == Nil) {
        return;
    }
    bbb_next_drawable_method = class_getInstanceMethod(metal_layer, @selector(nextDrawable));
    if (bbb_next_drawable_method == NULL) {
        return;
    }
    bbb_next_drawable_implementation = method_getImplementation(bbb_next_drawable_method);
    method_setImplementation(
        bbb_next_drawable_method, (IMP)bbb_observe_next_drawable
    );
}
