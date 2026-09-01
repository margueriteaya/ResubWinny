#ifndef RESUB_ARIBTLV_BRIDGE_H
#define RESUB_ARIBTLV_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RESUB_ARIBTLV_BRIDGE_ABI_VERSION 1

typedef struct resub_aribtlv_demuxer resub_aribtlv_demuxer;

typedef struct resub_aribtlv_subtitle_track {
    uint64_t track_id;
    uint32_t context_id;
    uint16_t packet_id;
    uint16_t component_tag;
    uint8_t tag;
    uint8_t info_version;
    uint8_t type;
    uint8_t format;
    uint8_t operation_mode;
    uint8_t timing_mode;
    uint8_t display_mode;
    uint8_t resolution;
    uint8_t compression_type;
    uint8_t has_start_mpu_sequence_number;
    uint32_t start_mpu_sequence_number;
    uint8_t has_reference_start_ntp;
    uint64_t reference_start_ntp;
    uint8_t reference_start_time_leap_indicator;
    const char *language;
} resub_aribtlv_subtitle_track;

typedef struct resub_aribtlv_subtitle_resource {
    uint8_t subsample_number;
    uint8_t data_type;
    const uint8_t *data;
    size_t size;
} resub_aribtlv_subtitle_resource;

typedef struct resub_aribtlv_caption_unit {
    uint64_t track_id;
    uint16_t component_tag;
    const uint8_t *data;
    size_t size;
    int64_t pts_value;
    uint32_t pts_timescale;
    uint64_t input_offset;
    uint8_t random_access;
    uint8_t discontinuity;
    uint32_t discontinuity_reasons;
    uint8_t has_timing_mode;
    uint8_t timing_mode;
    uint8_t has_operation_mode;
    uint8_t operation_mode;
    uint8_t has_display_mode;
    uint8_t display_mode;
    uint8_t has_compression_type;
    uint8_t compression_type;
    uint8_t has_mpu_sequence_number;
    uint32_t mpu_sequence_number;
    uint8_t has_reference_start_pts;
    int64_t reference_start_pts_value;
    uint32_t reference_start_pts_timescale;
    const resub_aribtlv_subtitle_resource *resources;
    size_t resource_count;
} resub_aribtlv_caption_unit;

typedef struct resub_aribtlv_error {
    int32_t code;
    uint64_t input_offset;
    uint8_t recoverable;
    const char *message;
} resub_aribtlv_error;

typedef struct resub_aribtlv_callbacks {
    size_t struct_size;
    void (*on_track)(void *opaque, const resub_aribtlv_subtitle_track *track);
    void (*on_caption)(void *opaque, const resub_aribtlv_caption_unit *unit);
    void (*on_error)(void *opaque, const resub_aribtlv_error *error);
} resub_aribtlv_callbacks;

uint32_t resub_aribtlv_bridge_abi_version(void);
resub_aribtlv_demuxer *resub_aribtlv_create(
    const resub_aribtlv_callbacks *callbacks, void *opaque);
void resub_aribtlv_destroy(resub_aribtlv_demuxer *demuxer);
int resub_aribtlv_push(
    resub_aribtlv_demuxer *demuxer, const uint8_t *data, size_t size);
int resub_aribtlv_flush(resub_aribtlv_demuxer *demuxer);
const char *resub_aribtlv_last_error(const resub_aribtlv_demuxer *demuxer);

#ifdef __cplusplus
}
#endif

#endif
