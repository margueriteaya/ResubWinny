/*
 * Copyright (C) 2021 magicxqq <xqq@xqq.im>. All rights reserved.
 *
 * This file is part of libaribcaption.
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

#include <cassert>
#include <limits>
#include <stdexcept>
#include "renderer/bitmap.hpp"

namespace aribcaption {

Image Bitmap::ToImage(Bitmap&& bmp) {
    Image image;

    image.width = bmp.width();
    image.height = bmp.height();
    image.stride = bmp.stride();
    image.pixel_format = bmp.pixel_format();
    image.bitmap = std::move(bmp.pixels);

    bmp.width_ = 0;
    bmp.height_ = 0;
    bmp.stride_ = 0;

    return image;
}

Bitmap Bitmap::FromImage(Image&& image) {
    Bitmap bitmap;

    bitmap.width_ = image.width;
    bitmap.height_ = image.height;
    bitmap.stride_ = image.stride;
    bitmap.pixel_format_ = image.pixel_format;

    bitmap.pixels = std::move(image.bitmap);

    return bitmap;
}

Bitmap::Bitmap(int width, int height, PixelFormat pixel_format) :
      width_(width), height_(height), pixel_format_(pixel_format) {
    assert(width > 0 && height > 0);
    assert(pixel_format == PixelFormat::kRGBA8888);

    if (width <= 0 || height <= 0 || pixel_format != PixelFormat::kRGBA8888) {
        throw std::invalid_argument("invalid bitmap dimensions or pixel format");
    }

    constexpr size_t kBytesPerPixel = sizeof(ColorRGBA);
    const size_t max_stride = static_cast<size_t>(std::numeric_limits<int>::max());
    if (static_cast<size_t>(width) > max_stride / kBytesPerPixel) {
        throw std::length_error("bitmap stride exceeds supported range");
    }

    size_t stride = static_cast<size_t>(width) * kBytesPerPixel;

    size_t remainder = stride % kAlignedTo;
    if (remainder) {
        size_t padding = kAlignedTo - remainder;
        if (stride > max_stride - padding) {
            throw std::length_error("bitmap stride exceeds supported range");
        }
        stride += padding;
    }

    const size_t rows = static_cast<size_t>(height);
    if (stride > pixels.max_size() / rows) {
        throw std::length_error("bitmap allocation exceeds supported range");
    }

    stride_ = static_cast<int>(stride);
    pixels.resize(stride * rows);
}

}  // namespace aribcaption
