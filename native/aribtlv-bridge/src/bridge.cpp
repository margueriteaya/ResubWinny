#include "resub_aribtlv_bridge.h"

#include <aribtlv/aribtlv.h>

#include <new>
#include <vector>

static_assert(ARIBTLV_C_API_VERSION == 6,
              "review the bridge before updating libaribtlv's C API");

struct resub_aribtlv_demuxer {
    aribtlv_demuxer *inner = nullptr;
    resub_aribtlv_callbacks callbacks{};
    void *opaque = nullptr;
};

namespace {

void on_track(void *opaque, const aribtlv_track_info *track) {
    auto *state = static_cast<resub_aribtlv_demuxer *>(opaque);
    if (!state || !track || track->kind != ARIBTLV_TRACK_SUBTITLE ||
        track->codec != ARIBTLV_CODEC_TTML || !track->subtitle ||
        !state->callbacks.on_track) {
        return;
    }
    const auto &source = *track->subtitle;
    const resub_aribtlv_subtitle_track event{
        track->track_id,
        track->context_id,
        track->packet_id,
        track->component_tag,
        source.tag,
        source.info_version,
        source.type,
        source.format,
        source.operation_mode,
        source.timing_mode,
        source.display_mode,
        source.resolution,
        source.compression_type,
        source.has_start_mpu_sequence_number,
        source.start_mpu_sequence_number,
        source.has_reference_start_ntp,
        source.reference_start_ntp,
        source.reference_start_time_leap_indicator,
        track->language,
    };
    state->callbacks.on_track(state->opaque, &event);
}

void on_access_unit(void *opaque, const aribtlv_access_unit *unit) {
    auto *state = static_cast<resub_aribtlv_demuxer *>(opaque);
    if (!state || !unit || unit->codec != ARIBTLV_CODEC_TTML ||
        !state->callbacks.on_caption) {
        return;
    }
    std::vector<resub_aribtlv_subtitle_resource> resources;
    resources.reserve(unit->subtitle_resource_count);
    for (size_t index = 0; index < unit->subtitle_resource_count; ++index) {
        const auto &resource = unit->subtitle_resources[index];
        resources.push_back({resource.subsample_number, resource.data_type,
                             resource.data, resource.size});
    }
    const resub_aribtlv_caption_unit event{
        unit->track_id,
        unit->component_tag,
        unit->data,
        unit->size,
        unit->pts.value,
        unit->pts.timescale,
        unit->input_offset,
        unit->random_access,
        unit->discontinuity,
        unit->discontinuity_reasons,
        unit->has_subtitle_timing_mode,
        unit->subtitle_timing_mode,
        unit->has_subtitle_operation_mode,
        unit->subtitle_operation_mode,
        unit->has_subtitle_display_mode,
        unit->subtitle_display_mode,
        unit->has_subtitle_compression_type,
        unit->subtitle_compression_type,
        unit->has_mpu_sequence_number,
        unit->mpu_sequence_number,
        unit->has_subtitle_reference_start_pts,
        unit->subtitle_reference_start_pts.value,
        unit->subtitle_reference_start_pts.timescale,
        resources.empty() ? nullptr : resources.data(),
        resources.size(),
    };
    state->callbacks.on_caption(state->opaque, &event);
}

void on_error(void *opaque, const aribtlv_error *error) {
    auto *state = static_cast<resub_aribtlv_demuxer *>(opaque);
    if (!state || !error || !state->callbacks.on_error) {
        return;
    }
    const resub_aribtlv_error event{static_cast<int32_t>(error->code),
                                    error->input_offset, error->recoverable,
                                    error->message};
    state->callbacks.on_error(state->opaque, &event);
}

}  // namespace

extern "C" uint32_t resub_aribtlv_bridge_abi_version(void) {
    return RESUB_ARIBTLV_BRIDGE_ABI_VERSION;
}

extern "C" resub_aribtlv_demuxer *resub_aribtlv_create(
    const resub_aribtlv_callbacks *callbacks, void *opaque) {
    if (!callbacks || callbacks->struct_size != sizeof(*callbacks)) {
        return nullptr;
    }
    auto *state = new (std::nothrow) resub_aribtlv_demuxer{};
    if (!state) {
        return nullptr;
    }
    state->callbacks = *callbacks;
    state->opaque = opaque;
    aribtlv_callbacks upstream{};
    aribtlv_callbacks_init(&upstream);
    upstream.on_track = on_track;
    upstream.on_access_unit = on_access_unit;
    upstream.on_error = on_error;
    aribtlv_config config{};
    aribtlv_config_init(&config);
    config.collect_application_resources = 0;
    state->inner = aribtlv_demuxer_create(&config, &upstream, state);
    if (!state->inner ||
        aribtlv_demuxer_set_subtitle_passthrough(state->inner, 1) != ARIBTLV_OK) {
        if (state->inner) {
            aribtlv_demuxer_destroy(state->inner);
        }
        delete state;
        return nullptr;
    }
    return state;
}

extern "C" void resub_aribtlv_destroy(resub_aribtlv_demuxer *demuxer) {
    if (!demuxer) {
        return;
    }
    aribtlv_demuxer_destroy(demuxer->inner);
    delete demuxer;
}

extern "C" int resub_aribtlv_push(resub_aribtlv_demuxer *demuxer,
                                   const uint8_t *data, size_t size) {
    if (!demuxer || !demuxer->inner) {
        return ARIBTLV_ERROR_INVALID_ARGUMENT;
    }
    return aribtlv_demuxer_push(demuxer->inner, data, size);
}

extern "C" int resub_aribtlv_flush(resub_aribtlv_demuxer *demuxer) {
    if (!demuxer || !demuxer->inner) {
        return ARIBTLV_ERROR_INVALID_ARGUMENT;
    }
    return aribtlv_demuxer_flush(demuxer->inner);
}

extern "C" const char *resub_aribtlv_last_error(
    const resub_aribtlv_demuxer *demuxer) {
    return demuxer && demuxer->inner
               ? aribtlv_demuxer_last_error(demuxer->inner)
               : "invalid libaribtlv demuxer";
}
