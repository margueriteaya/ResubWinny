#include "demuxer_test_support.hpp"

#include <array>
#include <cstring>

std::vector<std::uint8_t> type2_fixture() {
    const std::vector<std::uint16_t> dts_pts_offsets{0, 10, 20};
    const std::vector<std::uint16_t> pts_offsets{111, 222, 333};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets, 7));
    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(2, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(3, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(3, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    return stream;
}

void test_hevc_irap_detection_without_mmtp_rap() {
    auto stream = discovery_stream();
    const auto add_video = [&](const std::uint32_t sequence,
                               const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence, 100U << 16U, false, mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_video(1, {0, 0, 0, 2, 0x46, 0x01});
    add_video(2, {0, 0, 0, 3, 0x26, 0x01, 0x80});
    add_video(3, {0, 0, 0, 2, 0x46, 0x01});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    const auto video = std::find_if(
        sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(video != sink.access_units.end() && video->random_access,
          "HEVC IRAP NAL was not exposed as a random-access AU without MMTP RAP");
}

void test_reposition_drops_orphan_hevc_irap_continuation() {
    auto initial = discovery_stream();
    append_video_access_unit(initial, 1);

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(initial.data(), initial.size());
    demuxer.flush();
    const auto initial_access_unit_count = sink.access_units.size();

    auto restarted = discovery_stream();
    const auto add_video = [&](const std::uint32_t sequence, const bool rap,
                               const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence, 100U << 16U, rap,
                           mpu_payload(1, mfu)));
        restarted.insert(restarted.end(), packet.begin(), packet.end());
    };

    // A checkpoint may be inside a large IRAP picture. This type-21 CRA NAL is
    // a continuation slice, so neither its NAL type nor an MMTP RAP flag makes
    // it the head of a decodable random-access point.
    add_video(1, true, {0, 0, 0, 3, 0x2a, 0x01, 0x00});
    add_video(2, false, {0, 0, 0, 2, 0x50, 0x01}); // suffix SEI of old picture
    add_video(3, false, {0, 0, 0, 3, 0x2a, 0x01, 0x00});
    const auto complete_picture_offset = static_cast<std::uint64_t>(restarted.size());
    add_video(4, false, {0, 0, 0, 2, 0x4e, 0x01}); // prefix SEI of new picture
    add_video(5, false, {0, 0, 0, 3, 0x2a, 0x01, 0x80});

    constexpr std::uint64_t source_offset = 500000;
    demuxer.reposition(aribtlv::RepositionOptions{source_offset, true});
    demuxer.push(restarted.data(), restarted.size());
    demuxer.flush();

    const auto first_restarted = sink.access_units.begin() +
        static_cast<std::ptrdiff_t>(initial_access_unit_count);
    const auto restarted_video_count = std::count_if(
        first_restarted, sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    const auto video = std::find_if(
        first_restarted, sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(restarted_video_count == 1 && video != sink.access_units.end(),
          "reposition emitted an orphan HEVC IRAP continuation as an access unit");
    check(video->data == std::vector<std::uint8_t>(
              {0, 0, 1, 0x4e, 0x01, 0, 0, 1, 0x2a, 0x01, 0x80}) &&
              video->random_access && video->discontinuity &&
              video->input_offset == source_offset + complete_picture_offset,
          "reposition did not resume at the first complete HEVC IRAP picture");
}

void test_access_unit_restart_offset_is_snapshotted() {
    const auto pa = discovery_message();
    auto stream = signalling_tlv(1, 0, pa);
    const auto first_checkpoint = static_cast<std::uint64_t>(0);
    const auto add_video = [&](const std::uint32_t sequence,
                               const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence, 100U << 16U, false, mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_video(1, {0, 0, 0, 2, 0x46, 0x01});
    add_video(2, {0, 0, 0, 3, 0x26, 0x01, 0x80});
    const auto later_signalling_offset = static_cast<std::uint64_t>(stream.size());
    const auto later_signalling = signalling_tlv(2, 0, pa);
    stream.insert(stream.end(), later_signalling.begin(), later_signalling.end());
    add_video(3, {0, 0, 0, 2, 0x46, 0x01});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    const auto video = std::find_if(
        sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(video != sink.access_units.end() &&
              video->restart_offset == first_checkpoint &&
              video->input_offset < later_signalling_offset,
          "AU used signalling received after the AU began as its restart checkpoint");
}

void test_restart_offset_includes_timestamp_mapping_origin() {
    const auto timing_signalling = signalling_tlv(
        1, 0, video_discovery_message(1));
    const auto metadata_only_signalling = signalling_tlv(
        2, 0, video_discovery_message(std::nullopt));
    auto stream = timing_signalling;
    const auto later_signalling_offset = static_cast<std::uint64_t>(stream.size());
    stream.insert(stream.end(), metadata_only_signalling.begin(),
                  metadata_only_signalling.end());

    const auto add_video = [&](const std::uint32_t sequence,
                               const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence, 100U << 16U, false,
                           mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_video(1, {0, 0, 0, 2, 0x46, 0x01});
    add_video(2, {0, 0, 0, 3, 0x26, 0x01, 0x80});
    add_video(3, {0, 0, 0, 2, 0x46, 0x01});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    const auto video = std::find_if(
        sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(video != sink.access_units.end() && video->random_access &&
              video->restart_offset == 0 &&
              video->restart_offset < later_signalling_offset,
          "AU restart offset omitted the earlier timestamp mapping origin");

    TestSink restarted_sink;
    aribtlv::Demuxer restarted(restarted_sink);
    restarted.reposition(aribtlv::RepositionOptions{video->restart_offset, true});
    restarted.push(stream.data() + video->restart_offset,
                   stream.size() - static_cast<std::size_t>(video->restart_offset));
    restarted.flush();
    check(std::any_of(restarted_sink.access_units.begin(),
                      restarted_sink.access_units.end(), [](const auto& unit) {
                          return unit.codec == aribtlv::Codec::Hevc &&
                              unit.random_access;
                      }),
          "timestamp-origin restart checkpoint could not reproduce its RAP");
}

void test_extended_timestamp_indexed_by_sample_number() {
    // dts_pts_offsets[0] == 0 keeps the first AU's own PTS at the Demuxer's
    // presentation-timeline origin, so the remaining assertions can compare
    // directly against the raw per-AU descriptor offsets below. pts_offsets is
    // uniform, as required for pts_offset_type == 2 (see emit_access_unit()).
    const std::vector<std::uint16_t> dts_pts_offsets{0, 20, 30, 40};
    const std::vector<std::uint16_t> pts_offsets{111, 111, 111, 111};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(2, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    // sample_number 3 is intentionally never delivered, leaving a hole.
    add_mfu(4, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(4, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> video;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::Hevc) video.push_back(&unit);
    }
    check(video.size() == 3, "expected exactly three HEVC access units around the dropped AU");
    check(video[0]->dts.value == 0 && video[0]->pts.value == 0,
          "first access unit did not use its own sample_number offsets");
    check(video[1]->dts.value == 91 && video[1]->pts.value == 111,
          "second access unit did not use its own sample_number offsets");
    check(video[2]->dts.value == 293 && video[2]->pts.value == 333,
          "access unit after the dropped sample_number was shifted onto the wrong "
          "descriptor entry");
}

void test_mpu_au_count_mismatch_flags_discontinuity() {
    const std::vector<std::uint16_t> dts_pts_offsets{10, 20};
    const std::vector<std::uint16_t> pts_offsets{111, 222};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));
    const auto next_mpu_signalling = signalling_tlv(2, 0, video_discovery_message(2));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t mpu_sequence, const std::uint32_t sample_number,
                             const bool rap, const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(mpu_sequence, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    // MPU 1's descriptor declares two access units, but only one is delivered
    // before MPU 2 begins.
    add_mfu(1, 1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, 1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    stream.insert(stream.end(), next_mpu_signalling.begin(), next_mpu_signalling.end());
    add_mfu(2, 1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(2, 1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    check(std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
              return error.code == aribtlv::ErrorCode::Discontinuity && error.recoverable;
          }),
          "short MPU access-unit count did not raise a recoverable discontinuity error");
    const auto second_mpu_video = std::find_if(
        sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc &&
                unit.mpu_sequence_number == std::optional<std::uint32_t>{2};
        });
    check(second_mpu_video != sink.access_units.end() && second_mpu_video->discontinuity,
          "MPU access-unit count mismatch did not mark the next access unit discontinuous");
    check(second_mpu_video->discontinuity_reasons ==
              aribtlv::DiscontinuityReason::SourceDamage,
          "source damage was not distinguished from a controlled discontinuity");
    check(sink.damage_spans.size() == 1 && sink.damage_spans[0].recovered &&
              sink.damage_spans[0].recovery_random_access &&
              sink.damage_spans[0].track_id == second_mpu_video->track_id &&
              sink.damage_spans[0].recovery_time ==
                  std::optional<aribtlv::Timestamp>{second_mpu_video->pts},
          "source damage did not report the next random-access recovery point");
}

void test_non_timed_media_mfu_ignores_opaque_header_as_sample_number() {
    const std::vector<std::uint16_t> dts_pts_offsets{0, 20};
    const std::vector<std::uint16_t> pts_offsets{111, 111};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t item_id, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           non_timed_mpu_payload(1, mfu, item_id)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    // Non-timed MFUs carry an opaque item_id in the same header slot a timed
    // MFU's sample_number occupies. Large/out-of-range values here must not
    // be treated as descriptor indices; the parser must fall back to the
    // emission counter exactly as it did before sample_number indexing.
    add_mfu(99, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(99, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(1, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> video;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::Hevc) video.push_back(&unit);
    }
    check(video.size() == 2,
          "non-timed media MFUs were misindexed by their opaque header field");
    check(video[0]->dts.value == 0 && video[0]->pts.value == 0,
          "first non-timed access unit did not fall back to the emission counter");
    check(video[1]->dts.value == 91 && video[1]->pts.value == 111,
          "second non-timed access unit did not fall back to the emission counter");
}

void test_aac_extended_timestamp_indexed_by_sample_number() {
    const std::vector<std::uint16_t> dts_pts_offsets{0, 20, 30, 40};
    const std::vector<std::uint16_t> pts_offsets{111, 111, 111, 111};
    auto stream = signalling_tlv(
        1, 0, audio_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf310, sequence++, 100U << 16U, false,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, {0x11, 0x22});
    add_mfu(2, {0x33, 0x44});
    // sample_number 3 is intentionally never delivered, leaving a hole.
    add_mfu(4, {0x55, 0x66});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> audio;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::AacLatm) audio.push_back(&unit);
    }
    check(audio.size() == 3, "expected exactly three AAC access units around the dropped AU");
    check(audio[0]->dts.value == 0 && audio[0]->pts.value == 0,
          "first AAC access unit did not use its own sample_number offsets");
    check(audio[1]->dts.value == 91 && audio[1]->pts.value == 111,
          "second AAC access unit did not use its own sample_number offsets");
    check(audio[2]->dts.value == 293 && audio[2]->pts.value == 333,
          "AAC access unit after the dropped sample_number was shifted onto the wrong "
          "descriptor entry");
}

void test_out_of_order_sample_number_allows_decreasing_type_2_dts() {
    // The observed type-2 receiver recurrence is allowed to produce a
    // decreasing DTS when decode-order sample numbers arrive out of order.
    // This is timestamp data, not malformed input to discard.
    const std::vector<std::uint16_t> dts_pts_offsets{0, 20, 30, 40, 50};
    const std::vector<std::uint16_t> pts_offsets{111, 111, 111, 111, 111};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(4, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(4, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(2, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(5, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(5, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> video;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::Hevc) video.push_back(&unit);
    }
    check(video.size() == 4, "type-2 decreasing-DTS access unit was dropped");
    check(video[0]->dts.value == 0 && video[1]->dts.value == 293 &&
              video[2]->dts.value == 91 && video[3]->dts.value == 394,
          "type-2 receiver recurrence did not preserve its decreasing DTS output");
    check(std::none_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
              return error.message ==
                  "dropped access unit with a decreasing decode timestamp inside an MPU";
          }),
          "type-2 decreasing DTS was still diagnosed as malformed input");
}

void test_sample_number_change_starts_a_new_access_unit() {
    // Neither AU carries an AUD (NAL 35) or a parameter-set/prefix-SEI NAL,
    // and the second AU's VCL NAL has first_slice_segment_in_pic_flag CLEAR
    // (the top bit of its third byte is 0), so the other three boundary
    // terms all stay false. Only the sample_number change can split them.
    const std::vector<std::uint16_t> dts_pts_offsets{0, 20};
    const std::vector<std::uint16_t> pts_offsets{111, 111};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    // A freshly-installed video track waits for a RAP before emitting, so
    // the first AU must be delivered as MMTP random-access.
    add_mfu(1, true, {0, 0, 0, 3, 0x02, 0x01, 0x80}); // pending empty, first_slice irrelevant
    add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x00}); // first_slice_segment_in_pic_flag CLEAR

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> video;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::Hevc) video.push_back(&unit);
    }
    check(video.size() == 2,
          "sample_number change did not split two plain VCL NAL units into separate "
          "access units");
    check(video[0]->dts.value == 0 && video[0]->pts.value == 0,
          "first access unit did not carry its own descriptor entry");
    check(video[1]->dts.value == 91 && video[1]->pts.value == 111,
          "second access unit did not carry its own descriptor entry");
}

void test_pts_offset_type_2_uniform_matches_pts_offset_type_1() {
    // A pts_offset_type == 2 descriptor whose per-AU pts_offset is uniform must
    // produce identical timestamps to the equivalent pts_offset_type == 1
    // descriptor: TR-B39 fixes pts_offset_type at '01' and replicates a single
    // default_pts_offset across the MPU, so accumulating either one over a
    // decode-order prefix sums the same constant regardless of order (see the
    // citation above MmtpParser::emit_access_unit).
    const std::uint8_t au_count = 3;
    const std::vector<std::uint16_t> dts_pts_offsets(au_count, 0);
    const std::vector<std::uint16_t> pts_offsets(au_count, 3000);

    struct Emitted {
        std::int64_t dts = 0;
        std::int64_t pts = 0;
    };
    const auto run = [](std::vector<std::uint8_t> discovery) {
        auto stream = signalling_tlv(1, 0, std::move(discovery));
        std::uint32_t sequence = 1;
        const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                                 const std::vector<std::uint8_t>& mfu) {
            const auto packet = tlv_for_mmtp(
                1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                               mpu_payload(1, mfu, sample_number)));
            stream.insert(stream.end(), packet.begin(), packet.end());
        };
        add_mfu(1, true, {0, 0, 0, 2, 0x46, 0x01});
        add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
        add_mfu(2, false, {0, 0, 0, 2, 0x46, 0x01});
        add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
        add_mfu(3, false, {0, 0, 0, 2, 0x46, 0x01});
        add_mfu(3, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

        TestSink sink;
        aribtlv::Demuxer demuxer(sink);
        demuxer.push(stream.data(), stream.size());
        demuxer.flush();

        std::vector<Emitted> emitted;
        for (const auto& unit : sink.access_units) {
            if (unit.codec == aribtlv::Codec::Hevc) {
                emitted.push_back(Emitted{unit.dts.value, unit.pts.value});
            }
        }
        return emitted;
    };

    const auto type1 = run(video_discovery_message_with_au_count(1, au_count));
    const auto type2 = run(video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets));

    check(type1.size() == 3 && type2.size() == 3,
          "expected three access units from both the pts_offset_type 1 and 2 streams");
    for (std::size_t index = 0; index < type1.size(); ++index) {
        check(type1[index].dts == type2[index].dts && type1[index].pts == type2[index].pts,
              "uniform pts_offset_type == 2 produced different timestamps than the "
              "equivalent pts_offset_type == 1 descriptor");
    }
}

void test_pts_offset_type_2_non_uniform_uses_per_au_recurrence() {
    // The factory recurrence uses each descriptor entry in decode order:
    // AU0=(PTS 0,DTS -decode_offset), then PTS accumulates the preceding
    // pts_offset before calculating the following AU's DTS.
    const std::vector<std::uint16_t> dts_pts_offsets{0, 10, 20};
    const std::vector<std::uint16_t> pts_offsets{111, 222, 333};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_offsets(1, dts_pts_offsets, pts_offsets, 7));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(2, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(3, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(3, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    // Type-2 timestamp parsing is independent of TLV/MMTP chunking.
    for (const auto byte : stream) demuxer.push(&byte, 1);
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> video;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::Hevc) video.push_back(&unit);
    }
    check(video.size() == 3, "non-uniform pts_offset_type == 2 did not emit every AU");
    check(video[0]->pts.value == 0 && video[0]->dts.value == -7 &&
              video[1]->pts.value == 111 && video[1]->dts.value == 101 &&
              video[2]->pts.value == 333 && video[2]->dts.value == 313,
          "non-uniform pts_offset_type == 2 did not use the factory per-AU recurrence");
}

void test_truncated_type_2_descriptor_installs_no_partial_mapping() {
    auto stream = type2_fixture();
    constexpr std::array<std::uint8_t, 2> tag{0x80, 0x26};
    const auto descriptor = std::search(stream.begin(), stream.end(), tag.begin(), tag.end());
    check(descriptor != stream.end() && std::next(descriptor, 2) != stream.end(),
          "type-2 fixture did not contain its timestamp descriptor");
    ++*(descriptor + 2); // descriptor declares one byte beyond its available pair data

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(std::none_of(sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
              return unit.codec == aribtlv::Codec::Hevc;
          }), "truncated type-2 descriptor installed a partial timestamp mapping");
    check(std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
              return error.code == aribtlv::ErrorCode::MalformedInput && error.recoverable;
          }), "truncated type-2 descriptor lacked recoverable parse evidence");
}

void test_pts_offset_type_3_is_rejected() {
    // pts_offset_type == 3 is reserved by TR-B39 Table 34.1-72 and must not be
    // silently treated as though every access unit shares one decode timestamp.
    const std::vector<std::uint16_t> dts_pts_offsets{0, 20};
    auto stream = signalling_tlv(
        1, 0, video_discovery_message_with_reserved_pts_offset_type(1, dts_pts_offsets));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t sample_number, const bool rap,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence++, 100U << 16U, rap,
                           mpu_payload(1, mfu, sample_number)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(1, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_mfu(2, false, {0, 0, 0, 2, 0x46, 0x01});
    add_mfu(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    check(std::none_of(sink.access_units.begin(), sink.access_units.end(),
                       [](const auto& unit) { return unit.codec == aribtlv::Codec::Hevc; }),
          "reserved pts_offset_type == 3 still built a timestamp mapping and emitted "
          "access units");
    check(std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
              return error.code == aribtlv::ErrorCode::UnsupportedFeature && error.recoverable &&
                  error.message ==
                      "mpu_extended_timestamp_descriptor: pts_offset_type 3 is reserved by "
                      "TR-B39 Table 34.1-72 and defines no pts_offset semantics; skipping it";
          }),
          "reserved pts_offset_type == 3 did not raise the expected recoverable error");
}

void test_leap_second_insertion_corrects_presentation_timeline() {
    // MPU1 is normal (100s, indicator 0). MPU2 enters the leap window (101s,
    // indicator 1, "the day before"). MPU3 repeats MPU2's wire
    // mpu_presentation_time (101s again, the inserted duplicate second) but
    // is where the indicator switches 1->0, so it is where TR-B39 Appendix 1
    // section 2.1 says the +1s correction begins. MPU4 (102s, indicator 0) proves
    // the correction persists past the transition MPU.
    auto stream =
        signalling_tlv(1, 0, audio_discovery_message_with_leap(1, 100ULL << 32U, 0));
    const auto mpu2 =
        signalling_tlv(2, 0, audio_discovery_message_with_leap(2, 101ULL << 32U, 1));
    const auto mpu3 =
        signalling_tlv(3, 0, audio_discovery_message_with_leap(3, 101ULL << 32U, 0));
    const auto mpu4 =
        signalling_tlv(4, 0, audio_discovery_message_with_leap(4, 102ULL << 32U, 0));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t mpu_sequence,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf310, sequence++, 100U << 16U, false, mpu_payload(mpu_sequence, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, {0x11, 0x22});
    stream.insert(stream.end(), mpu2.begin(), mpu2.end());
    add_mfu(2, {0x33, 0x44});
    stream.insert(stream.end(), mpu3.begin(), mpu3.end());
    add_mfu(3, {0x55, 0x66});
    stream.insert(stream.end(), mpu4.begin(), mpu4.end());
    add_mfu(4, {0x77, 0x88});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> audio;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::AacLatm) audio.push_back(&unit);
    }
    check(audio.size() == 4, "leap-second insertion test did not produce four AAC access units");
    check(audio[0]->pts.value == 0 && audio[0]->dts.value == 0,
          "first access unit was not normalized to the presentation-timeline origin");
    check(audio[1]->pts.value == 180000 && audio[1]->dts.value == 180000,
          "step before the transition was not the normal one-second inter-MPU step");
    check(audio[2]->pts.value == 360000 && audio[2]->dts.value == 360000,
          "leap-second insertion was not corrected away at the 1->0 transition MPU");
    check(audio[3]->pts.value == 540000 && audio[3]->dts.value == 540000,
          "the +1s leap correction did not persist past the transition MPU");
}

void test_leap_second_deletion_corrects_presentation_timeline() {
    // MPU1 is normal (100s, indicator 0). MPU2 enters the deletion window
    // (101s, indicator 2). MPU3 jumps straight to 103s -- 102s is the
    // deleted wire second -- and is where the indicator switches 2->0, so
    // TR-B39 Appendix 1 section 2.2 says the -1s correction begins there. MPU4
    // (104s, indicator 0) proves the correction persists.
    auto stream =
        signalling_tlv(1, 0, audio_discovery_message_with_leap(1, 100ULL << 32U, 0));
    const auto mpu2 =
        signalling_tlv(2, 0, audio_discovery_message_with_leap(2, 101ULL << 32U, 2));
    const auto mpu3 =
        signalling_tlv(3, 0, audio_discovery_message_with_leap(3, 103ULL << 32U, 0));
    const auto mpu4 =
        signalling_tlv(4, 0, audio_discovery_message_with_leap(4, 104ULL << 32U, 0));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t mpu_sequence,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf310, sequence++, 100U << 16U, false, mpu_payload(mpu_sequence, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, {0x11, 0x22});
    stream.insert(stream.end(), mpu2.begin(), mpu2.end());
    add_mfu(2, {0x33, 0x44});
    stream.insert(stream.end(), mpu3.begin(), mpu3.end());
    add_mfu(3, {0x55, 0x66});
    stream.insert(stream.end(), mpu4.begin(), mpu4.end());
    add_mfu(4, {0x77, 0x88});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> audio;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::AacLatm) audio.push_back(&unit);
    }
    check(audio.size() == 4, "leap-second deletion test did not produce four AAC access units");
    check(audio[0]->pts.value == 0 && audio[0]->dts.value == 0,
          "first access unit was not normalized to the presentation-timeline origin");
    check(audio[1]->pts.value == 180000 && audio[1]->dts.value == 180000,
          "step before the transition was not the normal one-second inter-MPU step");
    check(audio[2]->pts.value == 360000 && audio[2]->dts.value == 360000,
          "leap-second deletion's two-second jump was not corrected at the 2->0 transition MPU");
    check(audio[3]->pts.value == 540000 && audio[3]->dts.value == 540000,
          "the -1s leap correction did not persist past the transition MPU");
}

void test_leap_indicator_zero_is_inert() {
    // The indicator stays 0 throughout, so the correction must never engage:
    // the emitted timing must match plain, unadjusted mpu_presentation_time
    // arithmetic exactly.
    auto stream =
        signalling_tlv(1, 0, audio_discovery_message_with_leap(1, 100ULL << 32U, 0));
    const auto mpu2 =
        signalling_tlv(2, 0, audio_discovery_message_with_leap(2, 101ULL << 32U, 0));
    const auto mpu3 =
        signalling_tlv(3, 0, audio_discovery_message_with_leap(3, 102ULL << 32U, 0));

    std::uint32_t sequence = 1;
    const auto add_mfu = [&](const std::uint32_t mpu_sequence,
                             const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf310, sequence++, 100U << 16U, false, mpu_payload(mpu_sequence, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_mfu(1, {0x11, 0x22});
    stream.insert(stream.end(), mpu2.begin(), mpu2.end());
    add_mfu(2, {0x33, 0x44});
    stream.insert(stream.end(), mpu3.begin(), mpu3.end());
    add_mfu(3, {0x55, 0x66});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    std::vector<const aribtlv::AccessUnit*> audio;
    for (const auto& unit : sink.access_units) {
        if (unit.codec == aribtlv::Codec::AacLatm) audio.push_back(&unit);
    }
    check(audio.size() == 3, "leap-indicator-zero test did not produce three AAC access units");
    check(audio[0]->pts.value == 0 && audio[0]->dts.value == 0,
          "leap indicator 0 changed the first access unit's timing");
    check(audio[1]->pts.value == 180000 && audio[1]->dts.value == 180000,
          "leap indicator 0 changed the second access unit's timing");
    check(audio[2]->pts.value == 360000 && audio[2]->dts.value == 360000,
          "leap indicator 0 changed the third access unit's timing");
}

void test_video_presentation_hint() {
    aribtlv::EventInfo event;
    event.title = "普通の番組";
    check(aribtlv::video_presentation_hint(event) ==
              aribtlv::VideoPresentationHint::Unknown,
          "an unmarked programme was incorrectly classified as HDR");

    event.title = "番組\xF0\x9F\x86\xA7";
    check(aribtlv::video_presentation_hint(event) ==
              aribtlv::VideoPresentationHint::Unknown,
          "free-form title text was treated as a structured HDR icon");

    event.hdr_programme_icon = true;
    check(aribtlv::video_presentation_hint(event) ==
              aribtlv::VideoPresentationHint::Hdr,
          "the structured HDR programme icon was not recognized");
}


} // namespace

int main(const int argc, char** argv) {
    if (argc == 2 && std::strcmp(argv[1], "--emit-type2-fixture") == 0) {
        const auto fixture = type2_fixture();
        std::cout.write(reinterpret_cast<const char*>(fixture.data()),
                        static_cast<std::streamsize>(fixture.size()));
        return std::cout ? 0 : 1;
    }
    test_hevc_irap_detection_without_mmtp_rap();
    test_reposition_drops_orphan_hevc_irap_continuation();
    test_access_unit_restart_offset_is_snapshotted();
    test_restart_offset_includes_timestamp_mapping_origin();
    test_extended_timestamp_indexed_by_sample_number();
    test_mpu_au_count_mismatch_flags_discontinuity();
    test_non_timed_media_mfu_ignores_opaque_header_as_sample_number();
    test_aac_extended_timestamp_indexed_by_sample_number();
    test_out_of_order_sample_number_allows_decreasing_type_2_dts();
    test_sample_number_change_starts_a_new_access_unit();
    test_pts_offset_type_2_uniform_matches_pts_offset_type_1();
    test_pts_offset_type_2_non_uniform_uses_per_au_recurrence();
    test_truncated_type_2_descriptor_installs_no_partial_mapping();
    test_pts_offset_type_3_is_rejected();
    test_leap_second_insertion_corrects_presentation_timeline();
    test_leap_second_deletion_corrects_presentation_timeline();
    test_leap_indicator_zero_is_inert();
    test_video_presentation_hint();
    std::cout << "timestamp/MPU tests passed\n";
    return 0;
}
