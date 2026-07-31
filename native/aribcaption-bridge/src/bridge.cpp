#include "arib_caption_bridge.h"

#include <new>
#include <cstring>
#include <vector>

#include <aribcaption/aribcaption.h>

struct acb_decoder {
    aribcc_context_t* context = nullptr;
    aribcc_decoder_t* decoder = nullptr;
    aribcc_renderer_t* renderer = nullptr;
    acb_rendered_image rendered = {};
    std::vector<uint8_t> rendered_pixels;
};

struct acb_drcs_data {
    acb_drcs_glyph glyph = {};
    std::vector<uint8_t> pixels;
};

struct acb_caption_event {
    std::vector<acb_region> regions;
    std::vector<acb_character> characters;
    std::vector<acb_drcs_data> drcs;
};

static void copy_string(char* destination, size_t capacity, const char* source) {
    if (!destination || capacity == 0) return;
    std::memset(destination, 0, capacity);
    if (source) std::strncpy(destination, source, capacity - 1);
}

static void copy_drcs(acb_caption_event* event, aribcc_caption_t* caption, uint32_t code) {
    for (const auto& existing : event->drcs) {
        if (existing.glyph.drcs_code == code) return;
    }
    auto* source = aribcc_drcsmap_get(caption->drcs_map, code);
    if (!source) return;
    acb_drcs_data target;
    target.glyph.drcs_code = code;
    aribcc_drcs_get_size(source, &target.glyph.width, &target.glyph.height);
    aribcc_drcs_get_depth(source, &target.glyph.depth, &target.glyph.depth_bits);
    target.glyph.alternative_codepoint = aribcc_drcs_get_alternative_ucs4(source);
    copy_string(target.glyph.md5, sizeof(target.glyph.md5), aribcc_drcs_get_md5(source));
    copy_string(target.glyph.alternative_text, sizeof(target.glyph.alternative_text),
                aribcc_drcs_get_alternative_text(source));
    uint8_t* pixels = nullptr;
    size_t pixel_count = 0;
    aribcc_drcs_get_pixels(source, &pixels, &pixel_count);
    if (pixels && pixel_count) target.pixels.assign(pixels, pixels + pixel_count);
    target.glyph.pixel_count = target.pixels.size();
    event->drcs.push_back(std::move(target));
}

acb_decoder* acb_decoder_create(void) {
    auto* handle = new (std::nothrow) acb_decoder();
    if (!handle) return nullptr;
    handle->context = aribcc_context_alloc();
    if (!handle->context) {
        delete handle;
        return nullptr;
    }
    handle->decoder = aribcc_decoder_alloc(handle->context);
    handle->renderer = aribcc_renderer_alloc(handle->context);
    if (!handle->decoder || !aribcc_decoder_initialize(handle->decoder,
            ARIBCC_ENCODING_SCHEME_ARIB_STD_B24_JIS,
            ARIBCC_CAPTIONTYPE_CAPTION,
            ARIBCC_PROFILE_A,
            ARIBCC_LANGUAGEID_FIRST) || !handle->renderer ||
        !aribcc_renderer_initialize(handle->renderer, ARIBCC_CAPTIONTYPE_CAPTION,
                                    ARIBCC_FONTPROVIDER_TYPE_AUTO,
                                    ARIBCC_TEXTRENDERER_TYPE_AUTO)) {
        if (handle->decoder) aribcc_decoder_free(handle->decoder);
        if (handle->renderer) aribcc_renderer_free(handle->renderer);
        aribcc_context_free(handle->context);
        delete handle;
        return nullptr;
    }
    aribcc_renderer_set_merge_region_images(handle->renderer, true);
    aribcc_renderer_set_stroke_width(handle->renderer, 2.0f);
    aribcc_renderer_set_replace_drcs(handle->renderer, false);
    aribcc_renderer_set_force_no_ruby(handle->renderer, false);
    aribcc_renderer_set_force_no_background(handle->renderer, false);
    const char* fonts[] = {"Rounded M+ 1m for ARIB", "sans-serif"};
    aribcc_renderer_set_default_font_family(handle->renderer, fonts, 2, true);
    return handle;
}

void acb_decoder_destroy(acb_decoder* handle) {
    if (!handle) return;
    aribcc_decoder_free(handle->decoder);
    aribcc_renderer_free(handle->renderer);
    aribcc_context_free(handle->context);
    delete handle;
}

int acb_decoder_feed(acb_decoder* handle, const uint8_t* data, size_t size,
                     int64_t pts_ms, acb_caption_summary* summary,
                     acb_caption_event** event) {
    if (!handle || !handle->decoder || !data || size == 0 || !summary) return 0;
    *summary = {};
    if (event) *event = nullptr;
    aribcc_caption_t caption = {};
    auto status = aribcc_decoder_decode(handle->decoder, data, size, pts_ms, &caption);
    summary->status = static_cast<int>(status);
    if (status != ARIBCC_DECODE_STATUS_GOT_CAPTION) return summary->status;

    summary->pts_ms = caption.pts;
    summary->wait_duration_ms = caption.wait_duration;
    summary->plane_width = caption.plane_width;
    summary->plane_height = caption.plane_height;
    summary->region_count = caption.region_count;
    handle->rendered = {};
    handle->rendered_pixels.clear();
    aribcc_renderer_set_frame_size(handle->renderer, caption.plane_width, caption.plane_height);
    aribcc_renderer_append_caption(handle->renderer, &caption);
    aribcc_render_result_t rendered = {};
    if (aribcc_renderer_render(handle->renderer, caption.pts, &rendered) == ARIBCC_RENDER_STATUS_GOT_IMAGE &&
        rendered.image_count > 0 && rendered.images && rendered.images[0].bitmap) {
        const auto& image = rendered.images[0];
        handle->rendered = {image.width, image.height, image.stride, image.dst_x, image.dst_y, image.bitmap_size};
        handle->rendered_pixels.assign(image.bitmap, image.bitmap + image.bitmap_size);
    }
    aribcc_render_result_cleanup(&rendered);
    auto* captured = event ? new (std::nothrow) acb_caption_event() : nullptr;
    if (event && !captured) {
        aribcc_caption_cleanup(&caption);
        return 0;
    }
    for (uint32_t region_index = 0; region_index < caption.region_count; region_index++) {
        auto& region = caption.regions[region_index];
        if (captured) {
            captured->regions.push_back({
                region.x, region.y, region.width, region.height,
                static_cast<uint8_t>(region.is_ruby),
                static_cast<uint32_t>(captured->characters.size()), region.char_count,
            });
        }
        summary->character_count += region.char_count;
        for (uint32_t char_index = 0; char_index < region.char_count; char_index++) {
            auto& source = region.chars[char_index];
            if (source.type == ARIBCC_CHARTYPE_DRCS) {
                summary->unresolved_drcs_count++;
            }
            if (captured) {
                acb_character target = {};
                target.type = static_cast<uint32_t>(source.type);
                target.codepoint = source.codepoint;
                target.pua_codepoint = source.pua_codepoint;
                target.drcs_code = source.drcs_code;
                target.x = source.x;
                target.y = source.y;
                target.width = source.char_width;
                target.height = source.char_height;
                target.horizontal_spacing = source.char_horizontal_spacing;
                target.vertical_spacing = source.char_vertical_spacing;
                target.horizontal_scale = source.char_horizontal_scale;
                target.vertical_scale = source.char_vertical_scale;
                target.text_color = source.text_color;
                target.back_color = source.back_color;
                target.stroke_color = source.stroke_color;
                target.style = static_cast<uint32_t>(source.style);
                target.enclosure_style = static_cast<uint32_t>(source.enclosure_style);
                std::memcpy(target.utf8, source.u8str, sizeof(target.utf8));
                captured->characters.push_back(target);
                if (source.type == ARIBCC_CHARTYPE_DRCS || source.type == ARIBCC_CHARTYPE_DRCS_REPLACED) {
                    copy_drcs(captured, &caption, source.drcs_code);
                }
            }
        }
    }
    aribcc_caption_cleanup(&caption);
    if (event) *event = captured;
    return summary->status;
}

int acb_decoder_get_rendered_image(const acb_decoder* handle, acb_rendered_image* image) {
    if (!handle || !image || handle->rendered_pixels.empty()) return 0;
    *image = handle->rendered;
    return 1;
}

size_t acb_decoder_copy_rendered_rgba(const acb_decoder* handle, uint8_t* destination, size_t capacity) {
    if (!handle) return 0;
    if (destination && capacity >= handle->rendered_pixels.size() && !handle->rendered_pixels.empty()) {
        std::memcpy(destination, handle->rendered_pixels.data(), handle->rendered_pixels.size());
    }
    return handle->rendered_pixels.size();
}

void acb_caption_event_destroy(acb_caption_event* event) { delete event; }

uint32_t acb_caption_event_region_count(const acb_caption_event* event) {
    return event ? static_cast<uint32_t>(event->regions.size()) : 0;
}

int acb_caption_event_region_at(const acb_caption_event* event, uint32_t index, acb_region* region) {
    if (!event || !region || index >= event->regions.size()) return 0;
    *region = event->regions[index];
    return 1;
}

uint32_t acb_caption_event_character_count(const acb_caption_event* event) {
    return event ? static_cast<uint32_t>(event->characters.size()) : 0;
}

int acb_caption_event_character_at(const acb_caption_event* event, uint32_t index,
                                   acb_character* character) {
    if (!event || !character || index >= event->characters.size()) return 0;
    *character = event->characters[index];
    return 1;
}

uint32_t acb_caption_event_drcs_count(const acb_caption_event* event) {
    return event ? static_cast<uint32_t>(event->drcs.size()) : 0;
}

int acb_caption_event_drcs_at(const acb_caption_event* event, uint32_t index,
                               acb_drcs_glyph* glyph) {
    if (!event || !glyph || index >= event->drcs.size()) return 0;
    *glyph = event->drcs[index].glyph;
    return 1;
}

size_t acb_caption_event_copy_drcs_pixels(const acb_caption_event* event, uint32_t index,
                                          uint8_t* destination, size_t capacity) {
    if (!event || index >= event->drcs.size()) return 0;
    const auto& pixels = event->drcs[index].pixels;
    if (destination && capacity >= pixels.size() && !pixels.empty()) {
        std::memcpy(destination, pixels.data(), pixels.size());
    }
    return pixels.size();
}
