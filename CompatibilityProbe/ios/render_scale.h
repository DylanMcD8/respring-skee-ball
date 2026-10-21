#pragma once
#include <math.h>

// Preserve 320x480 game coordinates. Render at the smallest whole-number
// scale covering the physical 2:3 viewport, then resolve to the screen.
static inline unsigned ChuzzleRenderScale(double pixelWidth, double pixelHeight) {
    double fit = fmin(pixelWidth / 320.0, pixelHeight / 480.0);
    if (!isfinite(fit) || fit <= 1.0) return 1;
    return (unsigned)fmin(8.0, ceil(fit));
}
