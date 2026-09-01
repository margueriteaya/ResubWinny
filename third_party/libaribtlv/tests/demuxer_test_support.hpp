#include <aribtlv/demuxer.hpp>
#include <aribtlv/recording.hpp>
#include <aribtlv/video_presentation.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <optional>
#include <string>
#include <vector>

namespace {

struct TestSink final : aribtlv::Sink {
    std::vector<aribtlv::ServiceInfo> services;
    std::vector<aribtlv::TrackInfo> tracks;
    std::vector<aribtlv::AccessUnit> access_units;
    std::vector<aribtlv::ApplicationServiceInfo> application_services;
    std::vector<aribtlv::LayoutConfiguration> layouts;
    std::vector<aribtlv::DataAssetInfo> data_assets;
    std::vector<aribtlv::TrackInfo> removed_tracks;
    std::vector<aribtlv::ApplicationServiceInfo> removed_application_services;
    std::vector<aribtlv::DataAssetInfo> removed_data_assets;
    std::vector<aribtlv::SignallingMessage> signalling_messages;
    std::vector<aribtlv::EventInfo> events;
    std::vector<aribtlv::MhSdtSnapshot> mh_sdt_snapshots;
    std::vector<aribtlv::MhTotInfo> mh_tot;
    std::vector<aribtlv::StreamEvent> stream_events;
    std::vector<aribtlv::ViewerParticipationNotification>
        viewer_participation_notifications;
    std::vector<aribtlv::ApplicationInfo> applications;
    std::vector<aribtlv::ApplicationInfo> removed_applications;
    std::vector<aribtlv::MptSnapshot> mpt_snapshots;
    std::vector<aribtlv::MhAitSnapshot> mh_ait_snapshots;
    std::vector<aribtlv::ServiceStateReset> service_resets;
    std::vector<aribtlv::DataTransmissionTable> data_transmission_tables;
    std::vector<aribtlv::Error> errors;
    std::vector<aribtlv::DamageSpan> damage_spans;
    void onService(const aribtlv::ServiceInfo& value) override { services.push_back(value); }
    void onTrack(const aribtlv::TrackInfo& value) override { tracks.push_back(value); }
    void onTrackRemoved(const aribtlv::TrackInfo& value) override {
        removed_tracks.push_back(value);
    }
    void onAccessUnit(aribtlv::AccessUnit&& value) override {
        access_units.push_back(std::move(value));
    }
    void onApplicationService(const aribtlv::ApplicationServiceInfo& value) override {
        application_services.push_back(value);
    }
    void onApplicationServiceRemoved(
        const aribtlv::ApplicationServiceInfo& value) override {
        removed_application_services.push_back(value);
    }
    void onLayoutConfiguration(const aribtlv::LayoutConfiguration& value) override {
        layouts.push_back(value);
    }
    void onDataAsset(const aribtlv::DataAssetInfo& value) override {
        data_assets.push_back(value);
    }
    void onDataAssetRemoved(const aribtlv::DataAssetInfo& value) override {
        removed_data_assets.push_back(value);
    }
    void onSignallingMessage(aribtlv::SignallingMessage&& value) override {
        signalling_messages.push_back(std::move(value));
    }
    void onEventInfo(const aribtlv::EventInfo& value) override { events.push_back(value); }
    void onMhSdtSnapshot(const aribtlv::MhSdtSnapshot& value) override {
        mh_sdt_snapshots.push_back(value);
    }
    void onMhTot(const aribtlv::MhTotInfo& value) override { mh_tot.push_back(value); }
    void onStreamEvent(const aribtlv::StreamEvent& value) override {
        stream_events.push_back(value);
    }
    void onViewerParticipationNotification(
        const aribtlv::ViewerParticipationNotification& value) override {
        viewer_participation_notifications.push_back(value);
    }
    void onApplication(const aribtlv::ApplicationInfo& value) override {
        applications.push_back(value);
    }
    void onApplicationRemoved(const aribtlv::ApplicationInfo& value) override {
        removed_applications.push_back(value);
    }
    void onMptSnapshot(const aribtlv::MptSnapshot& value) override {
        mpt_snapshots.push_back(value);
    }
    void onMhAitSnapshot(const aribtlv::MhAitSnapshot& value) override {
        mh_ait_snapshots.push_back(value);
    }
    void onServiceStateReset(const aribtlv::ServiceStateReset& value) override {
        service_resets.push_back(value);
    }
    void onDataTransmissionTable(aribtlv::DataTransmissionTable&& value) override {
        data_transmission_tables.push_back(std::move(value));
    }
    void onError(const aribtlv::Error& value) override { errors.push_back(value); }
    void onDamage(const aribtlv::DamageSpan& value) override {
        damage_spans.push_back(value);
    }
};

[[noreturn]] void fail(const std::string& message) {
    std::cerr << "FAIL: " << message << '\n';
    std::exit(1);
}

void check(const bool condition, const std::string& message) {
    if (!condition) fail(message);
}

std::vector<std::uint8_t> mmtp_signalling(const std::uint16_t packet_id,
                                          const std::uint32_t sequence) {
    return {
        0x00, 0x02,
        static_cast<std::uint8_t>(packet_id >> 8U), static_cast<std::uint8_t>(packet_id),
        0, 0, 0, 0,
        static_cast<std::uint8_t>(sequence >> 24U), static_cast<std::uint8_t>(sequence >> 16U),
        static_cast<std::uint8_t>(sequence >> 8U), static_cast<std::uint8_t>(sequence),
        0x00, 0x00, 0x00, 0x00,
    };
}

std::vector<std::uint8_t> compressed(const std::uint16_t context_id,
                                     const std::uint16_t packet_id,
                                     const std::uint32_t sequence) {
    auto mmtp = mmtp_signalling(packet_id, sequence);
    std::vector<std::uint8_t> result{
        static_cast<std::uint8_t>((context_id << 4U) >> 8U),
        static_cast<std::uint8_t>(context_id << 4U),
        0x61,
    };
    result.insert(result.end(), mmtp.begin(), mmtp.end());
    return result;
}

std::vector<std::uint8_t> tlv(const std::uint8_t type,
                              const std::vector<std::uint8_t>& payload) {
    std::vector<std::uint8_t> result{
        0x7f, type,
        static_cast<std::uint8_t>(payload.size() >> 8U),
        static_cast<std::uint8_t>(payload.size()),
    };
    result.insert(result.end(), payload.begin(), payload.end());
    return result;
}

std::vector<std::uint8_t> stream_for_contexts(const std::uint16_t first,
                                              const std::uint16_t second) {
    auto result = tlv(0x03, compressed(first, 0x8000, 1));
    const auto tail = tlv(0x03, compressed(second, 0x8000, 1));
    result.insert(result.end(), tail.begin(), tail.end());
    return result;
}

void test_split_at_every_boundary() {
    const auto data = stream_for_contexts(1, 2);
    for (std::size_t split = 0; split <= data.size(); ++split) {
        TestSink sink;
        aribtlv::Demuxer demuxer(sink);
        demuxer.push(data.data(), split);
        demuxer.push(data.data() + split, data.size() - split);
        demuxer.flush();
        check(sink.services.size() == 2, "TLV split changed discovered context count");
    }
}

void test_one_byte_input() {
    const auto data = stream_for_contexts(7, 8);
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    for (const auto byte : data) demuxer.push(&byte, 1);
    demuxer.flush();
    check(sink.services.size() == 2, "one-byte pushes did not match whole-stream parsing");
}

void test_garbage_recovery() {
    auto data = stream_for_contexts(1, 2);
    const auto third = stream_for_contexts(3, 4);
    data.insert(data.end(), {0xde, 0xad, 0x7f, 0x03, 0xff, 0xff, 0xbe, 0xef});
    data.insert(data.end(), third.begin(), third.end());

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.services.size() == 4, "parser did not recover after middle garbage");
    check(std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
        return error.code == aribtlv::ErrorCode::MalformedInput;
    }), "garbage recovery did not report a recoverable error");
}

void test_service_selection_and_reset() {
    const auto data = stream_for_contexts(10, 11);
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.selectService(11);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.services.size() == 1 && sink.services[0].context_id == 11,
          "service selection leaked another context");

    demuxer.reset();
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.services.size() == 2, "reset did not make selected service discoverable again");
}

void test_incomplete_flush() {
    auto data = stream_for_contexts(1, 2);
    data.resize(data.size() - 3);
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(!sink.errors.empty(), "flush did not report incomplete trailing data");
}

void test_mode_60_and_resource_limit() {
    auto mmtp = mmtp_signalling(0x8000, 1);
    std::vector<std::uint8_t> stream;
    for (const auto mode_and_size : std::vector<std::pair<std::uint8_t, std::size_t>>{
             {0x20, 20}, {0x21, 2}, {0x60, 42}, {0x61, 0}}) {
        std::vector<std::uint8_t> payload{0x12, 0x30, mode_and_size.first};
        payload.insert(payload.end(), mode_and_size.second, 0);
        payload.insert(payload.end(), mmtp.begin(), mmtp.end());
        const auto packet = tlv(0x03, payload);
        stream.insert(stream.end(), packet.begin(), packet.end());
    }

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.services.size() == 1 && sink.services[0].context_id == 0x123,
          "compressed-IP modes did not preserve their shared context ID");

    aribtlv::Limits limits;
    limits.max_resync_buffer = 16;
    TestSink limited_sink;
    aribtlv::Demuxer limited(limited_sink, limits);
    std::vector<std::uint8_t> garbage(128, 0x55);
    limited.push(garbage.data(), garbage.size());
    limited.flush();
    check(std::any_of(limited_sink.errors.begin(), limited_sink.errors.end(), [](const auto& error) {
        return error.code == aribtlv::ErrorCode::ResourceLimit;
    }), "TLV resynchronization buffer limit was not enforced");

    const auto unsupported_packet = tlv(0x03, {0x00, 0x10, 0x22});
    std::vector<std::uint8_t> noisy_stream;
    for (int index = 0; index < 100; ++index) {
        noisy_stream.insert(noisy_stream.end(), unsupported_packet.begin(), unsupported_packet.end());
    }
    TestSink noisy_sink;
    aribtlv::Demuxer noisy(noisy_sink);
    noisy.push(noisy_stream.data(), noisy_stream.size());
    noisy.flush();
    const auto unsupported_callbacks = std::count_if(
        noisy_sink.errors.begin(), noisy_sink.errors.end(), [](const auto& error) {
            return error.code == aribtlv::ErrorCode::MalformedInput;
        });
    check(unsupported_callbacks > 0 && unsupported_callbacks < 10,
          "identical recoverable errors were not rate-limited");
}

void append_u16(std::vector<std::uint8_t>& value, const std::size_t number) {
    value.push_back(static_cast<std::uint8_t>(number >> 8U));
    value.push_back(static_cast<std::uint8_t>(number));
}

void append_u32(std::vector<std::uint8_t>& value, const std::size_t number) {
    value.push_back(static_cast<std::uint8_t>(number >> 24U));
    value.push_back(static_cast<std::uint8_t>(number >> 16U));
    value.push_back(static_cast<std::uint8_t>(number >> 8U));
    value.push_back(static_cast<std::uint8_t>(number));
}

void descriptor(std::vector<std::uint8_t>& value, const std::uint16_t tag,
                const std::vector<std::uint8_t>& payload) {
    append_u16(value, tag);
    value.push_back(static_cast<std::uint8_t>(payload.size()));
    value.insert(value.end(), payload.begin(), payload.end());
}

void append_u64(std::vector<std::uint8_t>& value, const std::uint64_t number) {
    append_u32(value, static_cast<std::uint32_t>(number >> 32U));
    append_u32(value, static_cast<std::uint32_t>(number));
}

void timing_descriptors(std::vector<std::uint8_t>& value,
                        const std::uint32_t mpu_sequence,
                        const std::uint32_t timescale,
                        const std::uint8_t au_count = 1,
                        const std::uint64_t mpu_presentation_time = 100ULL << 32U,
                        const std::uint8_t leap_indicator = 0) {
    std::vector<std::uint8_t> timestamp;
    append_u32(timestamp, mpu_sequence);
    append_u64(timestamp, mpu_presentation_time);
    descriptor(value, 0x0001, timestamp);

    std::vector<std::uint8_t> extended{0x03};
    append_u32(extended, timescale);
    append_u16(extended, 3000);
    append_u32(extended, mpu_sequence);
    extended.push_back(static_cast<std::uint8_t>(leap_indicator << 6U));
    append_u16(extended, 0);
    extended.push_back(au_count);
    for (std::uint16_t index = 0; index < au_count; ++index) append_u16(extended, 0);
    descriptor(value, 0x8026, extended);
}

// Unlike timing_descriptors() above, this carries a distinct dts_pts_offset per
// access unit (pts_offset_type == 2), so tests can prove the descriptor is indexed
// by sample_number rather than by emission order. pts_offsets must be uniform across
// entries unless a test is specifically exercising the non-uniform rejection, since
// emit_access_unit() only accumulates it correctly when it is constant across the MPU.
void per_au_timing_descriptors(std::vector<std::uint8_t>& value,
                               const std::uint32_t mpu_sequence,
                               const std::uint32_t timescale,
                               const std::vector<std::uint16_t>& dts_pts_offsets,
                               const std::vector<std::uint16_t>& pts_offsets,
                               const std::uint16_t decoding_time_offset = 0) {
    std::vector<std::uint8_t> timestamp;
    append_u32(timestamp, mpu_sequence);
    append_u64(timestamp, 100ULL << 32U);
    descriptor(value, 0x0001, timestamp);

    std::vector<std::uint8_t> extended{0x05}; // timescale present, pts_offset_type == 2
    append_u32(extended, timescale);
    append_u32(extended, mpu_sequence);
    extended.push_back(0);
    append_u16(extended, decoding_time_offset);
    extended.push_back(static_cast<std::uint8_t>(dts_pts_offsets.size()));
    for (std::size_t index = 0; index < dts_pts_offsets.size(); ++index) {
        append_u16(extended, dts_pts_offsets[index]);
        append_u16(extended, pts_offsets[index]);
    }
    descriptor(value, 0x8026, extended);
}

// pts_offset_type == 3 is reserved by TR-B39 Table 34.1-72: only dts_pts_offset is
// present per access unit, with no pts_offset field at all.
void reserved_pts_offset_type_descriptors(std::vector<std::uint8_t>& value,
                                          const std::uint32_t mpu_sequence,
                                          const std::uint32_t timescale,
                                          const std::vector<std::uint16_t>& dts_pts_offsets) {
    std::vector<std::uint8_t> timestamp;
    append_u32(timestamp, mpu_sequence);
    append_u64(timestamp, 100ULL << 32U);
    descriptor(value, 0x0001, timestamp);

    std::vector<std::uint8_t> extended{0x07}; // timescale present, pts_offset_type == 3
    append_u32(extended, timescale);
    append_u32(extended, mpu_sequence);
    extended.push_back(0);
    append_u16(extended, 0);
    extended.push_back(static_cast<std::uint8_t>(dts_pts_offsets.size()));
    for (const auto dts_pts : dts_pts_offsets) append_u16(extended, dts_pts);
    descriptor(value, 0x8026, extended);
}

void asset(std::vector<std::uint8_t>& body, const std::uint16_t packet_id,
           const std::string& type, const std::vector<std::uint8_t>& descriptors) {
    body.push_back(0);
    append_u32(body, 0);
    body.push_back(2);
    append_u16(body, packet_id);
    body.insert(body.end(), type.begin(), type.end());
    body.push_back(0xfe);
    body.push_back(1);
    body.push_back(0);
    append_u16(body, packet_id);
    append_u16(body, descriptors.size());
    body.insert(body.end(), descriptors.begin(), descriptors.end());
}

std::vector<std::uint8_t> layout_configuration_table() {
    std::vector<std::uint8_t> body{
        1,       // number_of_loop
        2, 0, 2, // layout 2, main device, two regions
        0, 0, 0, 100, 100, 0,
        1, 10, 20, 90, 80, 3,
    };
    descriptor(body, 0x8abc, {0xde, 0xad});
    descriptor(body, 0x8002, {0x12, 0x34, 0x56});
    std::vector<std::uint8_t> table{0x81, 7};
    append_u16(table, body.size());
    table.insert(table.end(), body.begin(), body.end());
    return table;
}

std::vector<std::uint8_t> discovery_message(
    const std::uint8_t b60_transfer_characteristics = 5,
    const std::uint8_t hdr_wcg_idc = 2) {
    std::vector<std::uint8_t> program_descriptors;
    descriptor(program_descriptors, 0x8034,
               {0x1f, 0x1f, 0xf1, 0x00, 0xff, 0x02, 0x00, 0xff, 0x03,
                40, 0x00, 0xff, 0x04});

    std::vector<std::uint8_t> video_descriptors;
    descriptor(video_descriptors, 0x8011, {0x00, 0x00});
    descriptor(video_descriptors, 0x8000, {0x00, 0x01});
    descriptor(video_descriptors, 0x8abc, {0xde, 0xad, 0xbe});
    descriptor(video_descriptors, 0x800a,
               {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, hdr_wcg_idc});
    descriptor(video_descriptors, 0x8010,
               {0, 0, 0, 0,
                static_cast<std::uint8_t>(b60_transfer_characteristics << 4U),
                'j', 'p', 'n'});
    descriptor(video_descriptors, 0x8003,
               {0, 0, 0, 1, 2, 1, 0,
                0, 0, 0, 2, 3, 4, 2, 0xaa, 0xbb});
    timing_descriptors(video_descriptors, 1, 180000);

    std::vector<std::uint8_t> audio_descriptors;
    descriptor(audio_descriptors, 0x8011, {0x01, 0x10});
    descriptor(audio_descriptors, 0x8000, {0x10, 0x00});
    descriptor(audio_descriptors, 0x8000, {0x11, 0x01});
    descriptor(audio_descriptors, 0x8014,
               {0xf3, 0x03, 0x01, 0x10, 0x11, 0xff, 0x5f, 'j', 'p', 'n'});
    timing_descriptors(audio_descriptors, 1, 180000, 2);

    std::vector<std::uint8_t> subtitle_descriptors;
    descriptor(subtitle_descriptors, 0x8011, {0x12, 0x30});
    descriptor(subtitle_descriptors, 0x8020,
               {0x00, 0x20, 0x30, 0x08, 'j', 'p', 'n', 0x02, 0x2a, 0x10,
                0x00, 0x00, 0x00, 0x05,
                0x00, 0x00, 0x00, 0x64, 0x80, 0x00, 0x00, 0x00, 0x7f});

    std::vector<std::uint8_t> application_descriptors;
    descriptor(application_descriptors, 0x8011, {0x12, 0x40});
    descriptor(application_descriptors, 0x8003, {0, 0, 0, 9, 2, 1, 0});

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x65};
    append_u16(mpt_body, program_descriptors.size());
    mpt_body.insert(mpt_body.end(), program_descriptors.begin(), program_descriptors.end());
    mpt_body.push_back(4);
    asset(mpt_body, 0xf300, "hev1", video_descriptors);
    asset(mpt_body, 0xf310, "mp4a", audio_descriptors);
    asset(mpt_body, 0xf330, "stpp", subtitle_descriptors);
    asset(mpt_body, 0xf340, "aapp", application_descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    const auto lct = layout_configuration_table();
    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + lct.size() + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), lct.begin(), lct.end());
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

std::vector<std::uint8_t> video_discovery_message(
    const std::optional<std::uint32_t> mpu_sequence) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8011, {0x00, 0x00});
    descriptor(descriptors, 0x8010, {0, 0, 0, 0, 0, 'j', 'p', 'n'});
    if (mpu_sequence.has_value()) {
        timing_descriptors(descriptors, *mpu_sequence, 180000);
    }

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x65, 0x00, 0x00, 1};
    asset(mpt_body, 0xf300, "hev1", descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

// Sibling of video_discovery_message() that lets a fixture pick the pts_offset_type
// == 1 descriptor's access-unit count, for comparison against the pts_offset_type
// == 2 descriptors below.
[[maybe_unused]] std::vector<std::uint8_t> video_discovery_message_with_au_count(
    const std::uint32_t mpu_sequence, const std::uint8_t au_count) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8011, {0x00, 0x00});
    descriptor(descriptors, 0x8010, {0, 0, 0, 0, 0, 'j', 'p', 'n'});
    timing_descriptors(descriptors, mpu_sequence, 180000, au_count);

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x65, 0x00, 0x00, 1};
    asset(mpt_body, 0xf300, "hev1", descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

[[maybe_unused]] std::vector<std::uint8_t> video_discovery_message_with_offsets(
    const std::uint32_t mpu_sequence, const std::vector<std::uint16_t>& dts_pts_offsets,
    const std::vector<std::uint16_t>& pts_offsets,
    const std::uint16_t decoding_time_offset = 0) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8011, {0x00, 0x00});
    descriptor(descriptors, 0x8010, {0, 0, 0, 0, 0, 'j', 'p', 'n'});
    per_au_timing_descriptors(descriptors, mpu_sequence, 180000, dts_pts_offsets, pts_offsets,
                              decoding_time_offset);

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x65, 0x00, 0x00, 1};
    asset(mpt_body, 0xf300, "hev1", descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

[[maybe_unused]] std::vector<std::uint8_t> video_discovery_message_with_reserved_pts_offset_type(
    const std::uint32_t mpu_sequence, const std::vector<std::uint16_t>& dts_pts_offsets) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8011, {0x00, 0x00});
    descriptor(descriptors, 0x8010, {0, 0, 0, 0, 0, 'j', 'p', 'n'});
    reserved_pts_offset_type_descriptors(descriptors, mpu_sequence, 180000, dts_pts_offsets);

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x65, 0x00, 0x00, 1};
    asset(mpt_body, 0xf300, "hev1", descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

[[maybe_unused]] std::vector<std::uint8_t> audio_discovery_message_with_offsets(
    const std::uint32_t mpu_sequence, const std::vector<std::uint16_t>& dts_pts_offsets,
    const std::vector<std::uint16_t>& pts_offsets) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8011, {0x01, 0x10});
    descriptor(descriptors, 0x8014,
               {0xf3, 0x03, 0x01, 0x10, 0x11, 0xff, 0x5f, 'j', 'p', 'n'});
    per_au_timing_descriptors(descriptors, mpu_sequence, 180000, dts_pts_offsets, pts_offsets);

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x66, 0x00, 0x00, 1};
    asset(mpt_body, 0xf310, "mp4a", descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

// Sibling of audio_discovery_message_with_offsets() that lets a fixture pick
// the MPU's mpu_presentation_time and mpu_presentation_time_leap_indicator
// directly, for exercising the leap-second correction in emit_access_unit().
[[maybe_unused]] std::vector<std::uint8_t> audio_discovery_message_with_leap(
    const std::uint32_t mpu_sequence, const std::uint64_t mpu_presentation_time,
    const std::uint8_t leap_indicator) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8011, {0x01, 0x10});
    descriptor(descriptors, 0x8014,
               {0xf3, 0x03, 0x01, 0x10, 0x11, 0xff, 0x5f, 'j', 'p', 'n'});
    timing_descriptors(descriptors, mpu_sequence, 180000, 1, mpu_presentation_time,
                       leap_indicator);

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x66, 0x00, 0x00, 1};
    asset(mpt_body, 0xf310, "mp4a", descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

std::vector<std::uint8_t> audio_discovery_message() {
    auto audio_descriptors = [](const std::uint8_t component_type,
                                const std::uint16_t component_tag,
                                const bool main_component,
                                const bool multilingual = false) {
        std::vector<std::uint8_t> descriptors;
        std::vector<std::uint8_t> audio{
            0xf3,
            component_type,
            static_cast<std::uint8_t>(component_tag >> 8U),
            static_cast<std::uint8_t>(component_tag),
            0x11,
            0xff,
            static_cast<std::uint8_t>((multilingual ? 0x80U : 0U) |
                                      (main_component ? 0x40U : 0U) | 0x1fU),
            'j', 'p', 'n',
        };
        if (multilingual) audio.insert(audio.end(), {'e', 'n', 'g'});
        descriptor(descriptors, 0x8014, audio);
        timing_descriptors(descriptors, 1, 180000, 2);
        return descriptors;
    };

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x66, 0x00, 0x00, 3};
    asset(mpt_body, 0xe210, "mp4a", audio_descriptors(0x11, 0x0110, true));
    asset(mpt_body, 0xe275, "mp4a", audio_descriptors(0x09, 0x0011, false));
    asset(mpt_body, 0xe2aa, "mp4a", audio_descriptors(0x03, 0x0012, false, true));

    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

std::vector<std::uint8_t> application_control_message(
    const std::uint8_t section_number = 0,
    const std::uint8_t last_section_number = 0,
    const std::uint8_t version = 3,
    const bool include_application = true,
    const std::uint16_t application_type = 0x0011,
    const std::uint8_t control_code = 0x01) {
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8029,
               {0x05, 0x00, 0x01, 0x01, 0x02, 0x03,
                0xe1, 0x7f, 0x05});
    descriptor(descriptors, 0x802b,
               {'i', 'n', 'd', 'e', 'x', '.', 'h', 't', 'm', 'l'});

    std::vector<std::uint8_t> common_descriptors;
    std::vector<std::uint8_t> transport{0x00, 0x05, 0x05, 0x05};
    transport.insert(transport.end(), {'/', 'a', 'p', 'p', '/'});
    transport.push_back(0);
    descriptor(common_descriptors, 0x802a, transport);
    std::vector<std::uint8_t> unreferenced_transport{0x00, 0x05, 0x06, 0x07};
    unreferenced_transport.insert(unreferenced_transport.end(),
                                  {'/', 'i', 'g', 'n', 'o', 'r', 'e'});
    unreferenced_transport.push_back(0);
    descriptor(common_descriptors, 0x802a, unreferenced_transport);

    std::vector<std::uint8_t> applications;
    append_u16(applications, 0x1234);
    append_u32(applications, 0x01020304);
    applications.push_back(control_code);
    append_u16(applications, 0xf000U | descriptors.size());
    applications.insert(applications.end(), descriptors.begin(), descriptors.end());

    if (!include_application) applications.clear();
    std::vector<std::uint8_t> section{0x9c, 0x00, 0x00};
    append_u16(section, application_type);
    section.push_back(static_cast<std::uint8_t>(0xc1U | ((version & 0x1fU) << 1U)));
    section.push_back(section_number);
    section.push_back(last_section_number);
    append_u16(section, 0xf000U | common_descriptors.size());
    section.insert(section.end(), common_descriptors.begin(), common_descriptors.end());
    append_u16(section, 0xf000U | applications.size());
    section.insert(section.end(), applications.begin(), applications.end());
    append_u32(section, 0);
    const auto section_length = section.size() - 3;
    section[1] = static_cast<std::uint8_t>(0xf0U | (section_length >> 8U));
    section[2] = static_cast<std::uint8_t>(section_length);

    std::vector<std::uint8_t> message{0x80, 0x00, 0x00};
    append_u16(message, section.size());
    message.insert(message.end(), section.begin(), section.end());
    return message;
}

std::vector<std::uint8_t> mmtp_packet(const std::uint16_t packet_id,
                                      const std::uint32_t packet_sequence,
                                      const std::uint32_t delivery_timestamp,
                                      const bool random_access,
                                      const std::vector<std::uint8_t>& payload) {
    std::vector<std::uint8_t> result{static_cast<std::uint8_t>(random_access ? 1 : 0), 0,
        static_cast<std::uint8_t>(packet_id >> 8U), static_cast<std::uint8_t>(packet_id)};
    append_u32(result, delivery_timestamp); append_u32(result, packet_sequence);
    result.insert(result.end(), payload.begin(), payload.end()); return result;
}
std::vector<std::uint8_t> mpu_payload(const std::uint32_t mpu_sequence,
                                      const std::vector<std::uint8_t>& mfu,
                                      const std::uint32_t sample_number = 0) {
    std::vector<std::uint8_t> result; append_u16(result, 6 + 14 + mfu.size());
    result.insert(result.end(), {0x28, 0}); append_u32(result, mpu_sequence); append_u32(result, 0);
    append_u32(result, sample_number); append_u32(result, 0); result.insert(result.end(), {0, 0});
    result.insert(result.end(), mfu.begin(), mfu.end()); return result;
}
[[maybe_unused]] std::vector<std::uint8_t> non_timed_mpu_payload(
    const std::uint32_t mpu_sequence,
                                                const std::vector<std::uint8_t>& mfu,
                                                const std::uint32_t item_id = 0) {
    std::vector<std::uint8_t> result; append_u16(result, 6 + 4 + mfu.size());
    result.insert(result.end(), {0x20, 0}); append_u32(result, mpu_sequence); append_u32(result, item_id);
    result.insert(result.end(), mfu.begin(), mfu.end()); return result;
}
std::vector<std::uint8_t> tlv_for_mmtp(const std::uint16_t context_id,
                                       const std::vector<std::uint8_t>& mmtp) {
    std::vector<std::uint8_t> payload{static_cast<std::uint8_t>((context_id << 4U) >> 8U),
        static_cast<std::uint8_t>(context_id << 4U), 0x61};
    payload.insert(payload.end(), mmtp.begin(), mmtp.end()); return tlv(0x03, payload);
}

std::vector<std::uint8_t> data_transmission_message() {
    std::vector<std::uint8_t> table{0xa3, 0x00, 0x00, 0x2a, 0xff, 0xc9, 0x00, 0x00,
                                    0x01, '/', 0x00};
    append_u32(table, 0);
    const auto section_length = table.size() - 3;
    table[1] = static_cast<std::uint8_t>(0xf0U | (section_length >> 8U));
    table[2] = static_cast<std::uint8_t>(section_length);
    std::vector<std::uint8_t> message{0x80, 0x03, 0x00};
    append_u32(message, table.size());
    message.insert(message.end(), table.begin(), table.end());
    return message;
}

std::vector<std::uint8_t> signalling_mmtp(const std::uint32_t sequence,
                                          const std::uint8_t flags,
                                          const std::vector<std::uint8_t>& body,
                                          const std::uint16_t packet_id = 0xff02) {
    auto mmtp = mmtp_signalling(packet_id, 1);
    mmtp.resize(12);
    mmtp[8] = static_cast<std::uint8_t>(sequence >> 24U);
    mmtp[9] = static_cast<std::uint8_t>(sequence >> 16U);
    mmtp[10] = static_cast<std::uint8_t>(sequence >> 8U);
    mmtp[11] = static_cast<std::uint8_t>(sequence);
    mmtp.push_back(flags);
    mmtp.push_back(0);
    mmtp.insert(mmtp.end(), body.begin(), body.end());
    return mmtp;
}

std::vector<std::uint8_t> discovery_stream(
    const std::uint8_t b60_transfer_characteristics = 5,
    const std::uint8_t hdr_wcg_idc = 2) {
    const auto pa = discovery_message(b60_transfer_characteristics, hdr_wcg_idc);
    const auto mmtp = signalling_mmtp(1, 0, pa);
    std::vector<std::uint8_t> compressed_payload{0x00, 0x10, 0x61};
    compressed_payload.insert(compressed_payload.end(), mmtp.begin(), mmtp.end());
    const auto packet = tlv(0x03, compressed_payload);
    auto stream = packet;
    stream.insert(stream.end(), packet.begin(), packet.end());
    return stream;
}

std::vector<std::uint8_t> signalling_tlv(const std::uint32_t sequence,
                                         const std::uint8_t flags,
                                         const std::vector<std::uint8_t>& body,
                                         const std::uint16_t packet_id = 0xff02) {
    const auto mmtp = signalling_mmtp(sequence, flags, body, packet_id);
    std::vector<std::uint8_t> compressed_payload{0x00, 0x10, 0x61};
    compressed_payload.insert(compressed_payload.end(), mmtp.begin(), mmtp.end());
    return tlv(0x03, compressed_payload);
}

std::vector<std::uint8_t> mh_eit_message(const bool hdr = false) {
    const std::string title = hdr ? "録画された番組\xF0\x9F\x86\xA7" : "録画された番組";
    const std::string description = "番組概要";
    std::vector<std::uint8_t> short_event{'j', 'p', 'n',
        static_cast<std::uint8_t>(title.size())};
    short_event.insert(short_event.end(), title.begin(), title.end());
    append_u16(short_event, description.size());
    short_event.insert(short_event.end(), description.begin(), description.end());

    std::vector<std::uint8_t> descriptors;
    append_u16(descriptors, 0xf001);
    append_u16(descriptors, short_event.size());
    descriptors.insert(descriptors.end(), short_event.begin(), short_event.end());

    std::vector<std::uint8_t> extended{0x00, 'j', 'p', 'n'};
    std::vector<std::uint8_t> extended_items{0x04, 'C', 'a', 's', 't'};
    append_u16(extended_items, 5);
    extended_items.insert(extended_items.end(), {'A', 'l', 'i', 'c', 'e'});
    append_u16(extended, extended_items.size());
    extended.insert(extended.end(), extended_items.begin(), extended_items.end());
    append_u16(extended, 4);
    extended.insert(extended.end(), {'M', 'o', 'r', 'e'});
    append_u16(descriptors, 0xf002);
    append_u16(descriptors, extended.size());
    descriptors.insert(descriptors.end(), extended.begin(), extended.end());
    descriptor(descriptors, 0x8012, {0x12, 0x34});
    descriptor(descriptors, 0x8013, {'J', 'P', 'N', 0x04});
    descriptor(descriptors, 0x8014,
               {0x03, 0x03, 0x00, 0x10, 0x11, 0xff, 0x4e,
                'j', 'p', 'n', 'M', 'a', 'i', 'n'});
    descriptor(descriptors, 0x8016,
               {0x12, 0x34, 0x25, 0x9e, 0x8c, 0x00, 0x10, 0x0c,
                'S', 'e', 'r', 'i', 'e', 's'});

    std::vector<std::uint8_t> section{
        0x8b, 0xf0, 0x00,
        0x00, 0x65, // service_id 101
        0xc7,       // version 3, current_next=1
        0x00, 0x01,
        0x00, 0x0b, // tlv_stream_id 11
        0x00, 0x04, // original_network_id 4
        0x01, 0x8b,
        0x12, 0x34,
        0xc0, 0x79, 0x12, 0x45, 0x00, // 1993-10-13 12:45:00 JST
        0x01, 0x45, 0x30,
        static_cast<std::uint8_t>(0x80U | (descriptors.size() >> 8U)),
        static_cast<std::uint8_t>(descriptors.size()),
    };
    section.insert(section.end(), descriptors.begin(), descriptors.end());
    append_u32(section, 0);
    const auto section_length = section.size() - 3;
    section[1] = static_cast<std::uint8_t>(0xf0U | (section_length >> 8U));
    section[2] = static_cast<std::uint8_t>(section_length);

    std::vector<std::uint8_t> message{0x80, 0x00, 0x00};
    append_u16(message, section.size());
    message.insert(message.end(), section.begin(), section.end());
    return message;
}

std::vector<std::uint8_t> mh_sdt_message() {
    std::vector<std::uint8_t> service_descriptor{0x01, 0x03, 'N', 'H', 'K',
                                                  0x03, 'B', 'S', '4'};
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8019, service_descriptor);
    std::vector<std::uint8_t> section{0x9f, 0x00, 0x00};
    append_u16(section, 11);
    section.push_back(0xc7);
    section.push_back(0);
    section.push_back(0);
    append_u16(section, 4);
    section.push_back(0xff);
    append_u16(section, 101);
    section.push_back(0xff);
    append_u16(section, 0x8000U | descriptors.size());
    section.insert(section.end(), descriptors.begin(), descriptors.end());
    append_u32(section, 0);
    const auto section_length = section.size() - 3;
    section[1] = static_cast<std::uint8_t>(0xf0U | (section_length >> 8U));
    section[2] = static_cast<std::uint8_t>(section_length);
    std::vector<std::uint8_t> message{0x80, 0x00, 0x00};
    append_u16(message, section.size());
    message.insert(message.end(), section.begin(), section.end());
    return message;
}

std::vector<std::uint8_t> mh_tot_message() {
    std::vector<std::uint8_t> local_offset{
        'J', 'P', 'N', 0x05, 0x01, 0x00,
        0x9e, 0x8c, 0x13, 0x00, 0x00, 0x02, 0x00,
    };
    std::vector<std::uint8_t> descriptors;
    descriptor(descriptors, 0x8023, local_offset);
    std::vector<std::uint8_t> section{0xa1, 0x00, 0x00,
                                      0x9e, 0x8c, 0x12, 0x34, 0x56};
    append_u16(section, 0xf000U | descriptors.size());
    section.insert(section.end(), descriptors.begin(), descriptors.end());
    append_u32(section, 0);
    const auto section_length = section.size() - 3;
    section[1] = static_cast<std::uint8_t>(0x70U | (section_length >> 8U));
    section[2] = static_cast<std::uint8_t>(section_length);
    std::vector<std::uint8_t> message{0x80, 0x02, 0x00};
    append_u16(message, section.size());
    message.insert(message.end(), section.begin(), section.end());
    return message;
}

void append_video_access_unit(std::vector<std::uint8_t>& stream,
                              const std::uint32_t first_packet_sequence) {
    const auto add = [&](const std::uint32_t sequence, const bool rap,
                         const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(1, mmtp_packet(0xf300, sequence, 100U << 16U,
                                                         rap, mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add(first_packet_sequence, true, {0, 0, 0, 2, 0x46, 0x01});
    add(first_packet_sequence + 1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add(first_packet_sequence + 2, false, {0, 0, 0, 2, 0x46, 0x01});
}
