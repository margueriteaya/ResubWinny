#ifndef ARIB_CAPTION_BRIDGE_H
#define ARIB_CAPTION_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct acb_decoder acb_decoder;
typedef struct acb_caption_event acb_caption_event;

typedef struct acb_caption_summary {
    int status;
    int64_t pts_ms;
    int64_t wait_duration_ms;
    int plane_width;
    int plane_height;
    uint32_t region_count;
    uint32_t character_count;
    uint32_t unresolved_drcs_count;
} acb_caption_summary;

typedef struct acb_region {
    int x;
    int y;
    int width;
    int height;
    uint8_t is_ruby;
    uint32_t first_character;
    uint32_t character_count;
} acb_region;

typedef struct acb_character {
    uint32_t type;
    uint32_t codepoint;
    uint32_t pua_codepoint;
    uint32_t drcs_code;
    int x;
    int y;
    int width;
    int height;
    int horizontal_spacing;
    int vertical_spacing;
    float horizontal_scale;
    float vertical_scale;
    uint32_t text_color;
    uint32_t back_color;
    uint32_t stroke_color;
    uint32_t style;
    uint32_t enclosure_style;
    char utf8[8];
} acb_character;

typedef struct acb_drcs_glyph {
    uint32_t drcs_code;
    int width;
    int height;
    int depth;
    int depth_bits;
    uint32_t alternative_codepoint;
    size_t pixel_count;
    char md5[33];
    char alternative_text[8];
} acb_drcs_glyph;

typedef struct acb_rendered_image {
    int width;
    int height;
    int stride;
    int dst_x;
    int dst_y;
    size_t bitmap_size;
} acb_rendered_image;

acb_decoder* acb_decoder_create(void);
void acb_decoder_destroy(acb_decoder* decoder);

// data is the ARIB payload after the PES header. status is 0 on decoder error,
// 1 when no visual caption was produced, and 2 when summary contains a caption.
int acb_decoder_feed(acb_decoder* decoder, const uint8_t* data, size_t size,
                     int64_t pts_ms, acb_caption_summary* summary,
                     acb_caption_event** event);
int acb_decoder_get_rendered_image(const acb_decoder* decoder, acb_rendered_image* image);
size_t acb_decoder_copy_rendered_rgba(const acb_decoder* decoder, uint8_t* destination,
                                      size_t capacity);

void acb_caption_event_destroy(acb_caption_event* event);
uint32_t acb_caption_event_region_count(const acb_caption_event* event);
int acb_caption_event_region_at(const acb_caption_event* event, uint32_t index,
                                acb_region* region);
uint32_t acb_caption_event_character_count(const acb_caption_event* event);
int acb_caption_event_character_at(const acb_caption_event* event, uint32_t index,
                                   acb_character* character);
uint32_t acb_caption_event_drcs_count(const acb_caption_event* event);
int acb_caption_event_drcs_at(const acb_caption_event* event, uint32_t index,
                               acb_drcs_glyph* glyph);
size_t acb_caption_event_copy_drcs_pixels(const acb_caption_event* event, uint32_t index,
                                          uint8_t* destination, size_t capacity);

#ifdef __cplusplus
}
#endif

#endif
