#include "demuxer_test_support.hpp"

void test_independent_m2_sdt_and_tot() {
    auto stream = signalling_tlv(1, 0, mh_sdt_message(), 0x8004);
    const auto tot = signalling_tlv(1, 0, mh_tot_message(), 0x8005);
    stream.insert(stream.end(), tot.begin(), tot.end());
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.mh_sdt_snapshots.size() == 1 &&
              sink.mh_sdt_snapshots[0].tlv_stream_id == 11 &&
              sink.mh_sdt_snapshots[0].original_network_id == 4 &&
              sink.mh_sdt_snapshots[0].services.size() == 1 &&
              sink.mh_sdt_snapshots[0].services[0].service_id == 101 &&
              sink.mh_sdt_snapshots[0].services[0].provider_name == "NHK" &&
              sink.mh_sdt_snapshots[0].services[0].service_name == "BS4",
          "independent M2 MH-SDT did not produce a complete service snapshot");
    const auto expected_time =
        static_cast<std::int64_t>(86400 + 12 * 3600 + 34 * 60 + 56 - 9 * 3600) * 1000;
    check(sink.mh_tot.size() == 1 &&
              sink.mh_tot[0].time_unix_milliseconds == expected_time &&
              sink.mh_tot[0].local_time_offsets.size() == 1 &&
              sink.mh_tot[0].local_time_offsets[0].offset_minutes == 60 &&
              sink.mh_tot[0].local_time_offsets[0].next_offset_minutes == 120,
          "independent M2-short MH-TOT did not expose JST/local-offset state");
    check(sink.signalling_messages.size() == 2 &&
              sink.signalling_messages[0].message_id == 0x8000 &&
              sink.signalling_messages[1].message_id == 0x8002,
          "independent M2/M2-short messages were not exposed");
}
void test_mh_eit_program_events() {
    const auto message = mh_eit_message();
    auto stream = signalling_tlv(1, 0, message);
    const auto repeated = signalling_tlv(2, 0, message);
    stream.insert(stream.end(), repeated.begin(), repeated.end());

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.events.size() == 1, "repeated MH-EIT event was not deduplicated");
    const auto& event = sink.events[0];
    check(event.table_id == 0x8b && event.current_next && event.section_number == 0 &&
              event.service_id == 101 && event.tlv_stream_id == 11 &&
              event.original_network_id == 4 && event.event_id == 0x1234,
          "MH-EIT event identity was not parsed");
    check(event.start_time_unix_milliseconds == std::optional<std::int64_t>{750483900000LL} &&
              event.duration_seconds == std::optional<std::uint32_t>{6330},
          "MH-EIT MJD/BCD time was not converted from JST");
    check(event.running_status == 4 && !event.free_ca_mode && event.language == "jpn" &&
              event.title == "録画された番組" && !event.hdr_programme_icon &&
              event.description == "番組概要",
          "MH short-event descriptor was not parsed");
    check(event.extended_description == "More" && event.extended_items.size() == 1 &&
              event.extended_items[0].description == "Cast" &&
              event.extended_items[0].value == "Alice" &&
              event.genres.size() == 1 && event.genres[0].level1 == 1 &&
              event.genres[0].level2 == 2 && event.parental_ratings.size() == 1 &&
              event.parental_ratings[0].rating == 4,
          "MH extended/content/parental event descriptors were not parsed");
    check(event.audio_components.size() == 1 &&
              event.audio_components[0].audio.component_tag == 0x10 &&
              event.audio_components[0].audio.sample_rate == 48000 &&
              event.audio_components[0].text == "Main" && event.series.has_value() &&
              event.series->series_id == 0x1234 && event.series->episode_number == 1 &&
              event.series->last_episode_number == 12 && event.series->name == "Series",
          "MH audio-component/series event descriptors were not parsed");
}

void test_mh_eit_hdr_programme_icon() {
    const auto message = mh_eit_message(true);
    const auto stream = signalling_tlv(1, 0, message);
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.events.size() == 1 && sink.events.front().hdr_programme_icon &&
              aribtlv::video_presentation_hint(sink.events.front()) ==
                  aribtlv::VideoPresentationHint::Hdr,
          "MH-EIT structured HDR programme icon was not preserved");
}

std::vector<std::uint8_t> emt_message(const std::uint8_t version,
                                      const std::uint16_t message_id) {
    std::vector<std::uint8_t> descriptors;
    append_u16(descriptors, 0x8021);
    descriptors.push_back(17);
    append_u64(descriptors, 200ULL << 32U);
    append_u64(descriptors, 10ULL << 32U);
    descriptors.push_back(0x30); // leap=0, NPT advances at the UTC rate

    std::vector<std::uint8_t> event_payload;
    append_u16(event_payload, 0x001f); // group 1 + reserved nibble
    event_payload.push_back(0);        // immediate
    append_u64(event_payload, 0);
    event_payload.push_back(2);
    append_u16(event_payload, message_id);
    event_payload.insert(event_payload.end(), {0xde, 0xad});
    append_u16(descriptors, 0xf003);
    append_u16(descriptors, event_payload.size());
    descriptors.insert(descriptors.end(), event_payload.begin(), event_payload.end());

    std::vector<std::uint8_t> section{
        0xa6, 0xf0, 0x00,
        0x30, 0x01, // data_event_id 3, group 1
        static_cast<std::uint8_t>(0xc1U | ((version & 0x1fU) << 1U)),
        0x00, 0x00,
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

std::vector<std::uint8_t> viewer_participation_emt(
    const std::uint8_t version, const bool current_next = true) {
    std::vector<std::uint8_t> section{
        0xa6, 0xf0, 0x09,
        0xff, 0x00, // data_event_id 0xF, event_msg_group_id 0xF00
        static_cast<std::uint8_t>(0xc0U | ((version & 0x1fU) << 1U) |
                                  (current_next ? 1U : 0U)),
        0x00, 0x00,
    };
    append_u32(section, 0);
    std::vector<std::uint8_t> message{0x80, 0x00, 0x00};
    append_u16(message, section.size());
    message.insert(message.end(), section.begin(), section.end());
    return message;
}

void test_emt_stream_events() {
    auto stream = discovery_stream();
    const auto first = signalling_tlv(1, 0, emt_message(7, 0xb007), 0xff04);
    const auto repeated = signalling_tlv(2, 0, emt_message(7, 0xb007), 0xff04);
    const auto updated = signalling_tlv(3, 0, emt_message(7, 0xb008), 0xff04);
    stream.insert(stream.end(), first.begin(), first.end());
    stream.insert(stream.end(), repeated.begin(), repeated.end());
    stream.insert(stream.end(), updated.begin(), updated.end());

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.stream_events.size() == 2,
          "EMT messages were not deduplicated by identity and version");
    const auto& event = sink.stream_events.front();
    check(event.event_message_tag == 40 && event.data_event_id == 3 &&
              event.message_group_id == 1 && event.message_version == 7 &&
              event.time_mode == 0 && event.message_type == 2 &&
              event.raw_message_id == 0xb007 && event.message_id == 176,
          "EMT identity was not parsed with its MPT-signalled tag");
    check(event.utc_reference == std::optional<std::uint64_t>{200ULL << 32U} &&
              event.npt_reference == std::optional<std::uint64_t>{10ULL << 32U} &&
              event.private_data == std::vector<std::uint8_t>({0xde, 0xad}),
          "EMT timing reference or private data was not parsed");
}

void test_viewer_participation_notifications() {
    auto stream = signalling_tlv(1, 0, viewer_participation_emt(7), 0xff04);
    const auto repeated = signalling_tlv(2, 0, viewer_participation_emt(7), 0xff04);
    const auto updated = signalling_tlv(3, 0, viewer_participation_emt(8), 0xff04);
    const auto not_current = signalling_tlv(
        4, 0, viewer_participation_emt(9, false), 0xff04);
    stream.insert(stream.end(), repeated.begin(), repeated.end());
    stream.insert(stream.end(), updated.begin(), updated.end());
    stream.insert(stream.end(), not_current.begin(), not_current.end());

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.viewer_participation_notifications.size() == 2,
          "viewer-participation EMT was not deduplicated by table version");
    const auto& notification = sink.viewer_participation_notifications.front();
    check(notification.context_id == 1 && notification.source_packet_id == 0xff04 &&
              notification.event_message_tag == 0xff &&
              notification.data_event_id == 0x0f &&
              notification.message_group_id == 0x0f00 &&
              notification.version == 7 && notification.current_next &&
              notification.section_number == 0 &&
              notification.last_section_number == 0,
          "descriptor-less viewer-participation EMT identity was not exposed");
    check(sink.stream_events.empty(),
          "viewer-participation notification leaked into application StreamEvent");

    demuxer.reset();
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.viewer_participation_notifications.size() == 4,
          "full reset retained viewer-participation deduplication state");
}

void test_global_packet_state_budget() {
    const auto data = discovery_stream();
    aribtlv::Limits limits;
    limits.max_packet_states = 3; // one signalling PID plus two track states
    TestSink sink;
    aribtlv::Demuxer demuxer(sink, limits);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.tracks.size() == 2 &&
              std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
                  return error.code == aribtlv::ErrorCode::ResourceLimit;
              }),
          "global MMTP packet/track-state budget was not shared by signalling and tracks");
}

void test_signalling_fragmentation_aggregation_and_m2() {
    const auto pa = discovery_message();
    const auto first_end = pa.size() / 3;
    const auto middle_end = first_end * 2;
    auto first = signalling_tlv(10, 0x40,
        std::vector<std::uint8_t>(pa.begin(), pa.begin() + static_cast<std::ptrdiff_t>(first_end)));
    const auto middle = signalling_tlv(11, 0x80,
        std::vector<std::uint8_t>(pa.begin() + static_cast<std::ptrdiff_t>(first_end),
                                  pa.begin() + static_cast<std::ptrdiff_t>(middle_end)));
    const auto last = signalling_tlv(12, 0xc0,
        std::vector<std::uint8_t>(pa.begin() + static_cast<std::ptrdiff_t>(middle_end), pa.end()));
    first.insert(first.end(), middle.begin(), middle.end());
    first.insert(first.end(), last.begin(), last.end());
    first.insert(first.end(), last.begin(), last.end()); // duplicate is ignored
    TestSink fragmented_sink;
    aribtlv::Demuxer fragmented(fragmented_sink);
    fragmented.push(first.data(), first.size());
    fragmented.flush();
    check(fragmented_sink.tracks.size() == 3,
          "first/middle/last signalling fragments did not reassemble exactly once");

    std::vector<std::uint8_t> aggregate;
    append_u16(aggregate, pa.size());
    aggregate.insert(aggregate.end(), pa.begin(), pa.end());
    append_u16(aggregate, 4);
    aggregate.insert(aggregate.end(), {0x80, 0x03, 0x00, 0x00});
    auto aggregated_stream = signalling_tlv(20, 0x01, aggregate);
    const auto aggregate_tail = signalling_tlv(21, 0x01, aggregate);
    aggregated_stream.insert(aggregated_stream.end(), aggregate_tail.begin(), aggregate_tail.end());
    TestSink aggregated_sink;
    aribtlv::Demuxer aggregated(aggregated_sink);
    aggregated.push(aggregated_stream.data(), aggregated_stream.size());
    aggregated.flush();
    check(aggregated_sink.tracks.size() == 3,
          "aggregated signalling messages were not length-delimited and deduplicated");

    const auto mpt_start = static_cast<std::size_t>(8);
    std::vector<std::uint8_t> m2{0x80, 0x00, 0x00};
    append_u16(m2, pa.size() - mpt_start);
    m2.insert(m2.end(), pa.begin() + static_cast<std::ptrdiff_t>(mpt_start), pa.end());
    auto m2_stream = signalling_tlv(30, 0, m2);
    const auto m2_tail = signalling_tlv(31, 0, m2);
    m2_stream.insert(m2_stream.end(), m2_tail.begin(), m2_tail.end());
    TestSink m2_sink;
    aribtlv::Demuxer m2_demuxer(m2_sink);
    m2_demuxer.push(m2_stream.data(), m2_stream.size());
    m2_demuxer.flush();
    check(m2_sink.tracks.size() == 3, "M2 section message did not carry its MPT");

    auto gap_stream = signalling_tlv(40, 0x40,
        std::vector<std::uint8_t>(pa.begin(), pa.begin() + static_cast<std::ptrdiff_t>(first_end)));
    const auto gap_last = signalling_tlv(42, 0xc0,
        std::vector<std::uint8_t>(pa.begin() + static_cast<std::ptrdiff_t>(first_end), pa.end()));
    const auto recovered = signalling_tlv(43, 0, pa);
    gap_stream.insert(gap_stream.end(), gap_last.begin(), gap_last.end());
    gap_stream.insert(gap_stream.end(), recovered.begin(), recovered.end());
    TestSink gap_sink;
    aribtlv::Demuxer gap_demuxer(gap_sink);
    gap_demuxer.push(gap_stream.data(), gap_stream.size());
    gap_demuxer.flush();
    check(gap_sink.tracks.size() == 3 &&
              std::any_of(gap_sink.errors.begin(), gap_sink.errors.end(), [](const auto& error) {
                  return error.code == aribtlv::ErrorCode::Discontinuity;
              }),
          "signalling sequence gap did not discard the fragment and recover at a complete message");

    auto malformed_pa = pa;
    malformed_pa[10] = 0xff;
    malformed_pa[11] = 0xff;
    auto malformed_stream = signalling_tlv(50, 0, malformed_pa);
    const auto valid_after_malformed = signalling_tlv(51, 0, pa);
    malformed_stream.insert(malformed_stream.end(),
                            valid_after_malformed.begin(), valid_after_malformed.end());
    TestSink malformed_sink;
    aribtlv::Demuxer malformed_demuxer(malformed_sink);
    malformed_demuxer.push(malformed_stream.data(), malformed_stream.size());
    malformed_demuxer.flush();
    check(malformed_sink.tracks.size() == 3 &&
              std::any_of(malformed_sink.errors.begin(), malformed_sink.errors.end(), [](const auto& error) {
                  return error.code == aribtlv::ErrorCode::MalformedInput;
              }),
          "malformed nested MPT length damaged later signalling recovery");
}

void test_track_discovery_and_deduplication() {
    const auto data = discovery_stream();
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.tracks.size() == 3, "MPT did not discover exactly three supported tracks");
    check(sink.layouts.size() == 1 && sink.layouts[0].context_id == 1 &&
              sink.layouts[0].source_packet_id == 0xff02 &&
              sink.layouts[0].version == 7 &&
              sink.layouts[0].background_color_rgb ==
                  std::optional<std::uint32_t>{0x123456} &&
              sink.layouts[0].devices.size() == 1 &&
              sink.layouts[0].devices[0].layout_number == 2 &&
              sink.layouts[0].devices[0].device_id == 0 &&
              sink.layouts[0].devices[0].regions.size() == 2 &&
              sink.layouts[0].devices[0].regions[1].region_number == 1 &&
              sink.layouts[0].devices[0].regions[1].left_top_pos_x == 10 &&
              sink.layouts[0].devices[0].regions[1].left_top_pos_y == 20 &&
              sink.layouts[0].devices[0].regions[1].right_down_pos_x == 90 &&
              sink.layouts[0].devices[0].regions[1].right_down_pos_y == 80 &&
              sink.layouts[0].devices[0].regions[1].layer_order == 3,
          "LCT layout regions or background color were not exposed");
    check(sink.application_services.size() == 1 &&
              sink.application_services[0].application_format == 1 &&
              sink.application_services[0].document_resolution == 1 &&
              sink.application_services[0].default_ait &&
              sink.application_services[0].has_data_transmission_messages &&
              sink.application_services[0].ait_packet_id == 0xff02 &&
              sink.application_services[0].data_transmission_packet_id == 0xff03,
          "ARIB-HTML5 application service metadata was not parsed from the MPT");
    check(sink.application_services[0].event_message_locations.size() == 1 &&
              sink.application_services[0].event_message_locations[0].event_message_tag == 40 &&
              sink.application_services[0].event_message_locations[0].packet_id == 0xff04,
          "EMT tag/location metadata was not parsed from the MPT");
    check(sink.data_assets.size() == 1 &&
              sink.data_assets[0].packet_id == 0xf340 &&
              sink.data_assets[0].asset_type == "aapp" &&
              sink.data_assets[0].component_tag == 0x1240 &&
              sink.data_assets[0].presentation_regions ==
                  std::vector<aribtlv::MpuPresentationRegion>{{9, 2, 1}},
          "MMT application data asset was not exposed");
    check(sink.signalling_messages.size() == 1 &&
              sink.signalling_messages[0].message_id == 0x0000 &&
              sink.signalling_messages[0].packet_id == 0xff02,
          "completed MMTP signalling message was not exposed");
    check(sink.tracks[0].codec == aribtlv::Codec::Hevc && sink.tracks[0].timescale == 180000,
          "HEVC metadata was not parsed from MPT descriptors");
    check(sink.tracks[0].video.has_value() &&
              sink.tracks[0].video->hdr_wcg_idc == 2 &&
              sink.tracks[0].video->video_transfer_characteristics == 5,
          "HEVC colour signalling was not parsed from MPT descriptors");
    check(sink.tracks[0].presentation_regions ==
              std::vector<aribtlv::MpuPresentationRegion>{{1, 2, 1}, {2, 3, 4}},
          "MPU presentation-region descriptor was not exposed on the track");
    check(sink.tracks[0].asset_groups ==
              std::vector<aribtlv::AssetGroupInfo>{{0x00, 0x01}},
          "asset group metadata was not exposed on the video track");
    check(sink.tracks[1].codec == aribtlv::Codec::AacLatm && sink.tracks[1].language == "jpn",
          "AAC-LATM metadata was not parsed from MPT descriptors");
    check(sink.tracks[1].audio.has_value() &&
              sink.tracks[1].audio->channel_layout == aribtlv::AudioChannelLayout::Stereo &&
              sink.tracks[1].component_tag == 0x0110 &&
              sink.tracks[1].audio->component_tag == 0x0110 &&
              sink.tracks[1].audio->main_component &&
              sink.tracks[1].audio->sample_rate == 48000,
          "MH audio component metadata was not exposed on the audio track");
    check(sink.tracks[1].asset_groups ==
              std::vector<aribtlv::AssetGroupInfo>{{0x10, 0x00}, {0x11, 0x01}},
          "multiple asset group descriptors were not preserved on the audio track");
    check(sink.tracks[2].codec == aribtlv::Codec::Ttml &&
              sink.tracks[2].component_tag == 0x1230,
          "TTML metadata was not parsed from MPT descriptors");
    check(sink.tracks[2].timescale == 65536,
          "TTML without a timestamp descriptor did not use short-NTP timescale");
    check(sink.tracks[2].subtitle.has_value() &&
              sink.tracks[2].subtitle->operation_mode == 2 &&
              sink.tracks[2].subtitle->timing_mode == 2 &&
              sink.tracks[2].subtitle->display_mode == 10 &&
              sink.tracks[2].subtitle->resolution == 1 &&
              sink.tracks[2].subtitle->start_mpu_sequence_number == 5 &&
              sink.tracks[2].subtitle->reference_start_ntp ==
                  std::optional<std::uint64_t>{(100ULL << 32U) | 0x80000000ULL} &&
              sink.tracks[2].subtitle->reference_start_time_leap_indicator == 1,
          "ARIB B62 subtitle timing metadata was not exposed on the subtitle track");

    const auto stable_id = sink.tracks[0].track_id;
    demuxer.reset();
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.tracks.size() == 6 && sink.tracks[3].track_id == stable_id &&
              sink.layouts.size() == 2,
          "reset changed a track's Demuxer-lifetime stable identity");
}

void test_track_discovery_pq_signal() {
    const auto data = discovery_stream(4, 2);
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.tracks.size() == 3 && sink.tracks[0].video.has_value() &&
              sink.tracks[0].video->hdr_wcg_idc == 2 &&
              sink.tracks[0].video->video_transfer_characteristics == 4,
          "B60 PQ video descriptor packet was not propagated to TrackInfo");
}

// ARIB STD-B60 Table 9-3's TMD == 0010 branch of Additional_Arib_Subtitle_Info()
// is 9 bytes (reference_start_time plus the leap indicator/reserved byte); here
// it is cut back to 8, one byte short of the standard.
std::vector<std::uint8_t> truncated_subtitle_message() {
    std::vector<std::uint8_t> subtitle_descriptors;
    descriptor(subtitle_descriptors, 0x8011, {0x12, 0x30});
    descriptor(subtitle_descriptors, 0x8020,
               {0x00, 0x20, 0x30, 0x08, 'j', 'p', 'n', 0x02, 0x2a, 0x10,
                0x00, 0x00, 0x00, 0x05,
                0x00, 0x00, 0x00, 0x64, 0x80, 0x00, 0x00, 0x00});

    std::vector<std::uint8_t> mpt_body{0xfc, 2, 0x00, 0x65};
    append_u16(mpt_body, 0);
    mpt_body.push_back(1);
    asset(mpt_body, 0xf330, "stpp", subtitle_descriptors);
    std::vector<std::uint8_t> mpt{0x20, 8};
    append_u16(mpt, mpt_body.size());
    mpt.insert(mpt.end(), mpt_body.begin(), mpt_body.end());

    std::vector<std::uint8_t> pa{0x00, 0x00, 0x00};
    append_u32(pa, 1 + mpt.size());
    pa.push_back(0);
    pa.insert(pa.end(), mpt.begin(), mpt.end());
    return pa;
}

std::vector<std::uint8_t> truncated_subtitle_stream() {
    const auto pa = truncated_subtitle_message();
    const auto mmtp = signalling_mmtp(1, 0, pa);
    std::vector<std::uint8_t> compressed_payload{0x00, 0x10, 0x61};
    compressed_payload.insert(compressed_payload.end(), mmtp.begin(), mmtp.end());
    return tlv(0x03, compressed_payload);
}

void test_truncated_subtitle_reference_start_time_is_rejected() {
    const auto data = truncated_subtitle_stream();
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    // parse_mpt() rejects the whole MPT on a malformed asset descriptor, so the
    // subtitle track is never installed and the PA message never reaches onSignallingMessage.
    check(sink.tracks.empty(),
          "subtitle track was exposed despite a truncated reference_start_time block");
    check(sink.signalling_messages.empty(),
          "MPT with a truncated subtitle descriptor was still reported as a valid signalling "
          "message");
    check(!sink.errors.empty() && sink.errors[0].code == aribtlv::ErrorCode::MalformedInput &&
              sink.errors[0].recoverable,
          "truncated reference_start_time block did not raise a recoverable parse error");
}

void test_service_selection_clears_layout_state() {
    const auto data = discovery_stream();
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    demuxer.selectService(1);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.layouts.size() == 2,
          "service selection retained stale layout deduplication state");
}

void test_application_and_data_transmission_signalling() {
    auto stream = signalling_tlv(1, 0, application_control_message());
    const auto data_message = signalling_tlv(2, 0, data_transmission_message());
    stream.insert(stream.end(), data_message.begin(), data_message.end());

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(sink.applications.size() == 1 &&
              sink.applications[0].application_type == 0x0011 &&
              sink.applications[0].organization_id == 0x1234 &&
              sink.applications[0].application_id == 0x01020304 &&
              sink.applications[0].control_code == 0x01 &&
              sink.applications[0].version == 3 &&
              sink.applications[0].current_next &&
              sink.applications[0].section_number == 0 &&
              sink.applications[0].last_section_number == 0 &&
              sink.applications[0].application_descriptor_present &&
              sink.applications[0].profiles.size() == 1 &&
              sink.applications[0].profiles[0].application_profile == 0x0001 &&
              sink.applications[0].profiles[0].version_major == 1 &&
              sink.applications[0].profiles[0].version_minor == 2 &&
              sink.applications[0].profiles[0].version_micro == 3 &&
              sink.applications[0].service_bound &&
              sink.applications[0].visibility == 0x03 &&
              sink.applications[0].present_application_priority &&
              sink.applications[0].application_priority == 0x7f &&
              sink.applications[0].transport_protocol_labels ==
                  std::vector<std::uint8_t>{0x05} &&
              sink.applications[0].entry_path == "index.html" &&
              sink.applications[0].transports.size() == 2 &&
              sink.applications[0].transports[0].label == 0x05 &&
              sink.applications[0].transport_urls.size() == 1 &&
              sink.applications[0].transport_urls[0] == "/app/",
          "MH-AIT application identity, control, and location were not parsed");
    check(sink.data_transmission_tables.size() == 1 &&
              sink.data_transmission_tables[0].table_id == 0xa3 &&
              sink.data_transmission_tables[0].session_id == 0x2a &&
              sink.data_transmission_tables[0].version == 4 &&
              sink.data_transmission_tables[0].data.size() == 15,
          "data transmission table metadata was not exposed");
    check(sink.signalling_messages.size() == 2 &&
              sink.signalling_messages[0].message_id == 0x8000 &&
              sink.signalling_messages[1].message_id == 0x8003,
          "application signalling messages were not exposed after typed parsing");
}

void test_mpt_snapshot_removes_missing_service_state() {
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    const auto initial = discovery_stream();
    demuxer.push(initial.data(), initial.size());
    const auto ait = signalling_tlv(2, 0, application_control_message());
    demuxer.push(ait.data(), ait.size());
    check(sink.mpt_snapshots.size() == 1 && sink.applications.size() == 1,
          "initial MPT/MH-AIT snapshots were not committed once");

    const auto replacement = signalling_tlv(3, 0, video_discovery_message(1));
    demuxer.push(replacement.data(), replacement.size());
    check(sink.mpt_snapshots.size() == 2 && sink.removed_tracks.size() == 2 &&
              sink.removed_data_assets.size() == 1 &&
              sink.removed_application_services.size() == 1 &&
              sink.removed_applications.size() == 1,
          "complete MPT replacement did not retire every missing item atomically");

    const auto stale_ait = signalling_tlv(
        4, 0, application_control_message(0, 0, 4, true, 0x0011, 0x02));
    demuxer.push(stale_ait.data(), stale_ait.size());
    check(sink.mh_ait_snapshots.size() == 1 && sink.applications.size() == 1,
          "MPT descriptor removal did not stop the old MH-AIT route");
}

void test_mh_ait_snapshot_completion_empty_and_reposition() {
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);

    auto arib_single = signalling_tlv(
        1, 0, application_control_message(1, 1, 10));
    const auto arib_repeat = signalling_tlv(
        2, 0, application_control_message(1, 1, 10));
    arib_single.insert(arib_single.end(), arib_repeat.begin(), arib_repeat.end());
    demuxer.push(arib_single.data(), arib_single.size());
    check(sink.mh_ait_snapshots.size() == 1 && sink.applications.size() == 1 &&
              sink.mh_ait_snapshots.back().applications.size() == 1,
          "ARIB-HTML5 section 1/1 was incorrectly left waiting for section 0 (snapshots=" +
              std::to_string(sink.mh_ait_snapshots.size()) + ", applications=" +
              std::to_string(sink.applications.size()) + ", errors=" +
              std::to_string(sink.errors.size()) + ", signalling=" +
              std::to_string(sink.signalling_messages.size()) + ")");

    const auto empty = signalling_tlv(
        3, 0, application_control_message(1, 1, 11, false));
    demuxer.push(empty.data(), empty.size());
    check(sink.mh_ait_snapshots.size() == 2 &&
              sink.mh_ait_snapshots.back().applications.empty() &&
              sink.removed_applications.size() == 1,
          "empty MH-AIT snapshot did not retire the preceding application");

    const auto generic_last = signalling_tlv(
        4, 0, application_control_message(1, 1, 12, true, 0x0010));
    demuxer.push(generic_last.data(), generic_last.size());
    check(sink.mh_ait_snapshots.size() == 2,
          "generic multi-section MH-AIT committed without section 0");
    const auto generic_first = signalling_tlv(
        5, 0, application_control_message(0, 1, 12, false, 0x0010));
    demuxer.push(generic_first.data(), generic_first.size());
    check(sink.mh_ait_snapshots.size() == 3 &&
              sink.mh_ait_snapshots.back().applications.size() == 1,
          "out-of-order complete MH-AIT sub-table was not committed atomically");

    demuxer.reposition(aribtlv::RepositionOptions{0, true});
    auto historical = signalling_tlv(
        1, 0, application_control_message(1, 1, 2, true, 0x0011, 0x02));
    const auto historical_repeat = signalling_tlv(
        2, 0, application_control_message(1, 1, 2, true, 0x0011, 0x02));
    historical.insert(historical.end(), historical_repeat.begin(), historical_repeat.end());
    demuxer.push(historical.data(), historical.size());
    check(sink.mh_ait_snapshots.size() == 4 &&
              sink.mh_ait_snapshots.back().version == 2 &&
              sink.applications.back().control_code == 0x02,
          "first complete snapshot after reposition rejected a historical version rollback");
}

void test_service_state_reset_notifications() {
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.selectService(1);
    demuxer.reset();
    check(sink.service_resets.size() == 2 &&
              sink.service_resets[0].reason ==
                  aribtlv::ServiceStateResetReason::ServiceSelection &&
              sink.service_resets[1].reason ==
                  aribtlv::ServiceStateResetReason::FullReset,
          "service selection/full reset did not expose explicit reset ownership");
}

void test_dynamic_audio_layout_metadata() {
    const auto pa = audio_discovery_message();
    auto data = signalling_tlv(1, 0, pa);
    auto updated_pa = pa;
    const std::vector<std::uint8_t> first_audio_descriptor{0x80, 0x14, 0x0a, 0xf3, 0x11};
    const auto updated_component = std::search(updated_pa.begin(), updated_pa.end(),
                                               first_audio_descriptor.begin(),
                                               first_audio_descriptor.end());
    check(updated_component != updated_pa.end(), "audio update fixture has no 22.2ch descriptor");
    updated_component[4] = 0x09;
    const auto update = signalling_tlv(2, 0, updated_pa);
    data.insert(data.end(), update.begin(), update.end());

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(data.data(), data.size());
    demuxer.flush();
    check(sink.tracks.size() == 4,
          "three signalled audio tracks plus one metadata update were not reported");

    const auto find_layout = [&](const aribtlv::AudioChannelLayout layout) {
        return std::find_if(sink.tracks.begin(), sink.tracks.end(), [&](const auto& track) {
            return track.audio.has_value() && track.audio->channel_layout == layout;
        });
    };
    const auto surround22 = find_layout(aribtlv::AudioChannelLayout::Channels22_2);
    const auto surround51 = find_layout(aribtlv::AudioChannelLayout::Channels5_1);
    const auto stereo = find_layout(aribtlv::AudioChannelLayout::Stereo);
    check(surround22 != sink.tracks.end() && surround22->packet_id == 0xe210 &&
              surround22->component_tag == 0x0110 &&
              surround22->audio->component_tag == 0x0110 &&
              surround22->audio->main_component,
          "22.2ch track was not identified from its descriptor metadata");
    check(surround51 != sink.tracks.end() && surround51->packet_id == 0xe275 &&
              !surround51->audio->main_component,
          "5.1ch track was not identified independently of its packet ID");
    check(stereo != sink.tracks.end() && stereo->packet_id == 0xe2aa &&
              stereo->audio->es_multi_lingual &&
              stereo->audio->secondary_language == "eng",
          "stereo/multilingual track metadata was not parsed completely");
    check(sink.tracks.back().packet_id == 0xe210 &&
              sink.tracks.back().track_id == surround22->track_id &&
              sink.tracks.back().audio->channel_layout ==
                  aribtlv::AudioChannelLayout::Channels5_1,
          "audio descriptor update did not preserve track identity and emit replacement metadata");
}

std::vector<std::uint8_t> authenticated_mmtp_packet(
    const std::uint16_t packet_id, const std::uint32_t packet_sequence,
    const std::uint32_t delivery_timestamp, const bool random_access,
    const std::vector<std::uint8_t>& payload, const std::uint16_t declared_payload_size) {
    auto plain = mmtp_packet(packet_id, packet_sequence, delivery_timestamp,
                             random_access, payload);
    std::vector<std::uint8_t> result(plain.begin(), plain.begin() + 12);
    result[0] = static_cast<std::uint8_t>(result[0] | 0x02U);
    result.insert(result.end(), {
        0x00, 0x00, 0x00, 0x07, // multi-type extension
        0x80, 0x01, 0x00, 0x03, // final B61 extension, three-byte field
        0x02,                    // message authentication present
        static_cast<std::uint8_t>(declared_payload_size >> 8U),
        static_cast<std::uint8_t>(declared_payload_size),
    });
    result.insert(result.end(), payload.begin(), payload.end());
    result.insert(result.end(), {0xaa, 0xbb, 0xcc, 0xdd});
    return result;
}

std::vector<std::uint8_t> fragmented_mpu_payload(const std::uint32_t mpu_sequence,
                                                 const std::uint8_t fragmentation,
                                                 const std::vector<std::uint8_t>& piece) {
    auto result = mpu_payload(mpu_sequence, piece);
    result[2] = static_cast<std::uint8_t>(0x28U | (fragmentation << 1U));
    return result;
}

void test_authenticated_mmtp_payload_bounds() {
    const auto media = mpu_payload(1, {0x11, 0x22});
    auto valid_stream = discovery_stream();
    const auto valid_packet = tlv_for_mmtp(
        1, authenticated_mmtp_packet(0xf310, 1, 100U << 16U, true,
                                     media, static_cast<std::uint16_t>(media.size())));
    valid_stream.insert(valid_stream.end(), valid_packet.begin(), valid_packet.end());

    TestSink valid_sink;
    aribtlv::Demuxer valid_demuxer(valid_sink);
    valid_demuxer.push(valid_stream.data(), valid_stream.size());
    valid_demuxer.flush();
    check(std::any_of(valid_sink.access_units.begin(), valid_sink.access_units.end(),
                      [](const auto& unit) {
                          return unit.codec == aribtlv::Codec::AacLatm;
                      }),
          "B61 message-authentication code was treated as MMTP media payload");

    auto invalid_stream = discovery_stream();
    const auto invalid_packet = tlv_for_mmtp(
        1, authenticated_mmtp_packet(0xf310, 1, 100U << 16U, true,
                                     media, static_cast<std::uint16_t>(media.size() + 32)));
    invalid_stream.insert(invalid_stream.end(), invalid_packet.begin(), invalid_packet.end());
    TestSink invalid_sink;
    aribtlv::Demuxer invalid_demuxer(invalid_sink);
    invalid_demuxer.push(invalid_stream.data(), invalid_stream.size());
    invalid_demuxer.flush();
    check(std::any_of(invalid_sink.errors.begin(), invalid_sink.errors.end(),
                      [](const auto& error) {
                          return error.code == aribtlv::ErrorCode::MalformedInput;
                      }),
          "out-of-bounds authenticated MMTP payload length was accepted");
}

void test_recording_scanner_uses_demux_metadata_and_bounds_time() {
    auto stream = discovery_stream();
    append_video_access_unit(stream, 1);

    aribtlv::RecordingScanner scanner;
    check(scanner.push(stream.data(), stream.size()),
          "recording scanner rejected a valid recording");
    const auto& result = scanner.finish();
    check(result.complete() && result.video_packet_id == 0xf300 &&
              result.first_presentation_time.has_value() &&
              result.last_presentation_time.has_value() &&
              result.seek_points.size() == 1,
          "recording scanner did not expose the selected video timeline and RAP");
    check(scanner.seekFromStart({0, 1000000}).has_value(),
          "recording scanner could not locate the recording start RAP");
    check(!scanner.seekFromStart({1, 1000000}).has_value(),
          "recording scanner returned the final RAP for a target beyond the recording end");

    aribtlv::RecordingScanOptions options;
    options.video_packet_id = 0xf301;
    aribtlv::RecordingScanner wrong_video(options);
    check(wrong_video.push(stream.data(), stream.size()) &&
              wrong_video.finish().failure == aribtlv::RecordingScanFailure::NoVideo,
          "recording scanner ignored the requested video packet id");
}

void test_codec_output_and_timeline() {
    auto stream = discovery_stream();
    auto add_media = [&](const std::uint16_t packet_id, const std::uint32_t packet_sequence,
                         const bool rap, const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(packet_id, packet_sequence, 100U << 16U, rap,
                           mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };

    add_media(0xf300, 1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_media(0xf300, 2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_media(0xf300, 3, false, {0, 0, 0, 2, 0x46, 0x01});
    add_media(0xf310, 1, true, {0x11, 0x22});
    add_media(0xf310, 2, false, {0x33, 0x44});
    add_media(0xf330, 1, true, {0x30, 0x01, 0x00, 0x01, 0x04, 0x00, 0x03,
                                0x10, 0x00, 0x03, 'a', 'b', 'c'});
    add_media(0xf330, 2, false, {0x30, 0x01, 0x01, 0x01, 0x10, 0x00, 0x03,
                                 'd', 'e', 'f'});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();

    check(sink.access_units.size() >= 4, "supported codec MFUs did not produce access units");
    const auto video = std::find_if(sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
        return unit.codec == aribtlv::Codec::Hevc;
    });
    const auto audio = std::find_if(sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
        return unit.codec == aribtlv::Codec::AacLatm;
    });
    const auto subtitle = std::find_if(sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
        return unit.codec == aribtlv::Codec::Ttml;
    });
    check(video != sink.access_units.end() &&
              video->data == std::vector<std::uint8_t>({0, 0, 1, 0x46, 0x01,
                                                        0, 0, 1, 0x02, 0x01, 0x80}),
          "HEVC MFUs were not assembled into one Annex-B access unit");
    check(audio != sink.access_units.end() &&
              audio->data == std::vector<std::uint8_t>({0x56, 0xe0, 0x02, 0x11, 0x22}),
          "AAC MFU was not wrapped in a valid LOAS header");
    const auto second_audio = std::find_if(audio + 1, sink.access_units.end(), [](const auto& unit) {
        return unit.codec == aribtlv::Codec::AacLatm;
    });
    check(second_audio != sink.access_units.end() && second_audio->pts.value == 3000,
          "multi-AU timestamp offsets were not applied in presentation order");
    check(subtitle != sink.access_units.end() &&
              subtitle->data == std::vector<std::uint8_t>({'a', 'b', 'c'}),
          "TTML document was not separated from its resource subsamples");
    check(subtitle != sink.access_units.end() && subtitle->component_tag == 0x1230 &&
              subtitle->subtitle_timing_mode == std::optional<std::uint8_t>{2} &&
              subtitle->subtitle_operation_mode == std::optional<std::uint8_t>{2} &&
              subtitle->subtitle_display_mode == std::optional<std::uint8_t>{10} &&
              subtitle->subtitle_compression_type == std::optional<std::uint8_t>{0},
          "TTML access unit omitted component, timing, or B60 control metadata");
    check(subtitle != sink.access_units.end() && subtitle->subtitle_resources.size() == 1 &&
              subtitle->subtitle_resources[0].subsample_number == 1 &&
              subtitle->subtitle_resources[0].data_type == 1 &&
              subtitle->subtitle_resources[0].data == std::vector<std::uint8_t>({'d', 'e', 'f'}),
          "TTML resource subsample metadata was not preserved");
    check(video->pts.value == 0 && video->dts.value == 0,
          "first selected media timestamp was not normalized to zero");

    TestSink passthrough_sink;
    aribtlv::Demuxer passthrough_demuxer(passthrough_sink);
    passthrough_demuxer.selectTrack(aribtlv::TrackKind::Subtitle,
                                    std::numeric_limits<std::uint64_t>::max());
    passthrough_demuxer.setSubtitlePassthroughEnabled(true);
    passthrough_demuxer.push(stream.data(), stream.size());
    passthrough_demuxer.flush();
    check(std::any_of(passthrough_sink.access_units.begin(),
                      passthrough_sink.access_units.end(), [](const auto& unit) {
                          return unit.codec == aribtlv::Codec::Ttml;
                      }),
          "subtitle passthrough did not bypass selected-track filtering");
}

void test_timestamp_overflow_rejection() {
    auto pa = discovery_message();
    const std::vector<std::uint8_t> timestamp_pattern{0x00, 0x01, 0x0c, 0, 0, 0, 1};
    const auto first_timestamp = std::search(pa.begin(), pa.end(),
                                             timestamp_pattern.begin(), timestamp_pattern.end());
    check(first_timestamp != pa.end(), "test fixture has no video timestamp descriptor");
    const auto second_timestamp = std::search(first_timestamp + 1, pa.end(),
                                              timestamp_pattern.begin(), timestamp_pattern.end());
    check(second_timestamp != pa.end(), "test fixture has no audio timestamp descriptor");
    const auto timestamp_index = static_cast<std::size_t>(second_timestamp - pa.begin());
    for (std::size_t index = 0; index < 4; ++index) pa[timestamp_index + 7 + index] = 0xff;
    for (std::size_t index = 4; index < 8; ++index) pa[timestamp_index + 7 + index] = 0x00;

    const std::vector<std::uint8_t> extended_tag{0x80, 0x26};
    const auto audio_extended = std::search(second_timestamp, pa.end(),
                                            extended_tag.begin(), extended_tag.end());
    check(audio_extended != pa.end(), "test fixture has no audio extended timestamp descriptor");
    const auto extended_index = static_cast<std::size_t>(audio_extended - pa.begin());
    for (std::size_t index = 0; index < 4; ++index) pa[extended_index + 4 + index] = 0xff;

    auto stream = signalling_tlv(1, 0, pa);
    const auto repeated_signalling = signalling_tlv(2, 0, pa);
    stream.insert(stream.end(), repeated_signalling.begin(), repeated_signalling.end());
    auto add_media = [&](const std::uint16_t packet_id, const std::uint32_t sequence,
                         const bool rap, const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(packet_id, sequence, 100U << 16U, rap, mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_media(0xf300, 1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_media(0xf300, 2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_media(0xf300, 3, false, {0, 0, 0, 2, 0x46, 0x01});
    add_media(0xf310, 1, true, {0x11, 0x22});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    check(std::count_if(sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
              return unit.codec == aribtlv::Codec::AacLatm;
          }) == 0 &&
              std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
                  return error.code == aribtlv::ErrorCode::Discontinuity;
              }),
          "timestamp normalization overflow was not rejected recoverably");
}

void test_track_selection_clears_incomplete_media() {
    const auto discovery = discovery_stream();
    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(discovery.data(), discovery.size());
    demuxer.flush();
    const auto video_track = sink.tracks[0].track_id;

    const auto first_fragment = tlv_for_mmtp(
        1, mmtp_packet(0xf300, 1, 100U << 16U, true,
                       fragmented_mpu_payload(1, 1, {0, 0, 0, 3, 0x02})));
    const auto boundary = tlv(0xff, {});
    auto partial_stream = first_fragment;
    partial_stream.insert(partial_stream.end(), boundary.begin(), boundary.end());
    demuxer.push(partial_stream.data(), partial_stream.size());

    demuxer.selectTrack(aribtlv::TrackKind::Video, video_track);

    const auto stale_last = tlv_for_mmtp(
        1, mmtp_packet(0xf300, 2, 100U << 16U, false,
                       fragmented_mpu_payload(1, 3, {0x01, 0x80})));
    auto stale_stream = stale_last;
    stale_stream.insert(stale_stream.end(), boundary.begin(), boundary.end());
    demuxer.push(stale_stream.data(), stale_stream.size());

    const auto next_mpu_signalling = signalling_tlv(
        10, 0, video_discovery_message(2));
    demuxer.push(next_mpu_signalling.data(), next_mpu_signalling.size());
    auto add_video = [&](const std::uint32_t sequence, const bool rap,
                         const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence, 100U << 16U, rap, mpu_payload(2, mfu)));
        demuxer.push(packet.data(), packet.size());
    };
    add_video(10, true, {0, 0, 0, 2, 0x46, 0x01});
    add_video(11, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_video(12, false, {0, 0, 0, 2, 0x46, 0x01});
    demuxer.flush();

    check(std::count_if(sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
              return unit.codec == aribtlv::Codec::Hevc;
          }) == 1,
          "track selection retained stale fragmented media or failed to resume at a fresh RAP");
}

void test_fragmented_signalling_restart_offset() {
    const auto pa = discovery_message();
    const auto first_end = pa.size() / 3;
    const auto middle_end = first_end * 2;
    const auto prefix = tlv(0xff, {});
    auto stream = prefix;
    const auto first = signalling_tlv(
        10, 0x40,
        std::vector<std::uint8_t>(pa.begin(),
                                  pa.begin() + static_cast<std::ptrdiff_t>(first_end)));
    const auto middle = signalling_tlv(
        11, 0x80,
        std::vector<std::uint8_t>(pa.begin() + static_cast<std::ptrdiff_t>(first_end),
                                  pa.begin() + static_cast<std::ptrdiff_t>(middle_end)));
    const auto last = signalling_tlv(
        12, 0xc0,
        std::vector<std::uint8_t>(pa.begin() + static_cast<std::ptrdiff_t>(middle_end),
                                  pa.end()));
    stream.insert(stream.end(), first.begin(), first.end());
    stream.insert(stream.end(), middle.begin(), middle.end());
    stream.insert(stream.end(), last.begin(), last.end());

    auto add_video = [&](const std::uint32_t sequence, const bool rap,
                         const std::vector<std::uint8_t>& mfu) {
        const auto packet = tlv_for_mmtp(
            1, mmtp_packet(0xf300, sequence, 100U << 16U, rap, mpu_payload(1, mfu)));
        stream.insert(stream.end(), packet.begin(), packet.end());
    };
    add_video(1, true, {0, 0, 0, 2, 0x46, 0x01});
    add_video(2, false, {0, 0, 0, 3, 0x02, 0x01, 0x80});
    add_video(3, false, {0, 0, 0, 2, 0x46, 0x01});

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(stream.data(), stream.size());
    demuxer.flush();
    const auto video = std::find_if(sink.access_units.begin(), sink.access_units.end(),
                                    [](const auto& unit) {
                                        return unit.codec == aribtlv::Codec::Hevc;
                                    });
    check(video != sink.access_units.end() && video->restart_offset == prefix.size() &&
              video->input_offset > video->restart_offset,
          "AU restart offset did not retain the first fragmented signalling packet");
}

void test_reposition_preserves_timeline_and_absolute_offsets() {
    auto initial = discovery_stream();
    append_video_access_unit(initial, 1);

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(initial.data(), initial.size());
    demuxer.flush();
    const auto initial_video = std::find_if(
        sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(initial_video != sink.access_units.end() && initial_video->pts.value == 0,
          "initial video did not establish the recording timeline");
    const auto initial_video_index =
        static_cast<std::size_t>(initial_video - sink.access_units.begin());
    const auto original_track_callbacks = sink.tracks.size();

    auto shifted_pa = discovery_message();
    const std::vector<std::uint8_t> timestamp_pattern{0x00, 0x01, 0x0c, 0, 0, 0, 1};
    const auto video_timestamp = std::search(shifted_pa.begin(), shifted_pa.end(),
                                             timestamp_pattern.begin(), timestamp_pattern.end());
    check(video_timestamp != shifted_pa.end(), "shifted fixture has no video timestamp");
    const auto ntp_index = static_cast<std::size_t>(video_timestamp - shifted_pa.begin()) + 7;
    shifted_pa[ntp_index + 0] = 0;
    shifted_pa[ntp_index + 1] = 0;
    shifted_pa[ntp_index + 2] = 0;
    shifted_pa[ntp_index + 3] = 101;
    shifted_pa[ntp_index + 4] = 0;
    shifted_pa[ntp_index + 5] = 0;
    shifted_pa[ntp_index + 6] = 0;
    shifted_pa[ntp_index + 7] = 0;

    auto shifted = signalling_tlv(100, 0, shifted_pa);
    const auto repeated = signalling_tlv(101, 0, shifted_pa);
    const auto latest_checkpoint_offset = static_cast<std::uint64_t>(shifted.size());
    shifted.insert(shifted.end(), repeated.begin(), repeated.end());
    append_video_access_unit(shifted, 1000);

    constexpr std::uint64_t source_offset = 500000;
    demuxer.reposition(aribtlv::RepositionOptions{source_offset, true});
    demuxer.push(shifted.data(), shifted.size());
    demuxer.flush();

    const auto second_video = std::find_if(
        sink.access_units.begin() + static_cast<std::ptrdiff_t>(initial_video_index + 1),
        sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(second_video != sink.access_units.end() && second_video->pts.value == 180000,
          "reposition reset the recording timeline instead of preserving it");
    check(second_video->restart_offset == source_offset + latest_checkpoint_offset &&
              second_video->input_offset > second_video->restart_offset,
          "reposition did not preserve absolute source offsets");
    check(second_video->discontinuity,
          "first access unit after reposition was not marked discontinuous");
    check(second_video->discontinuity_reasons ==
              aribtlv::DiscontinuityReason::Reposition &&
              sink.damage_spans.empty(),
          "reposition was incorrectly reported as damaged source media");
    check(sink.tracks.size() == original_track_callbacks,
          "reposition re-emitted unchanged track metadata");
}

void test_track_selection_preserves_timeline_and_waits_for_rap() {
    auto initial = discovery_stream();
    append_video_access_unit(initial, 1);

    TestSink sink;
    aribtlv::Demuxer demuxer(sink);
    demuxer.push(initial.data(), initial.size());
    demuxer.flush();
    const auto initial_video = std::find_if(
        sink.access_units.begin(), sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(initial_video != sink.access_units.end() && initial_video->pts.value == 0,
          "initial video did not establish the track-switch timeline");
    const auto initial_video_index =
        static_cast<std::size_t>(initial_video - sink.access_units.begin());
    const auto video_track = initial_video->track_id;

    demuxer.selectTrack(aribtlv::TrackKind::Video, video_track);

    auto shifted_pa = discovery_message();
    const std::vector<std::uint8_t> timestamp_pattern{0x00, 0x01, 0x0c, 0, 0, 0, 1};
    const auto video_timestamp = std::search(shifted_pa.begin(), shifted_pa.end(),
                                             timestamp_pattern.begin(), timestamp_pattern.end());
    check(video_timestamp != shifted_pa.end(), "track-switch fixture has no video timestamp");
    const auto ntp_index = static_cast<std::size_t>(video_timestamp - shifted_pa.begin()) + 7;
    shifted_pa[ntp_index + 0] = 0;
    shifted_pa[ntp_index + 1] = 0;
    shifted_pa[ntp_index + 2] = 0;
    shifted_pa[ntp_index + 3] = 101;
    shifted_pa[ntp_index + 4] = 0;
    shifted_pa[ntp_index + 5] = 0;
    shifted_pa[ntp_index + 6] = 0;
    shifted_pa[ntp_index + 7] = 0;

    auto shifted = signalling_tlv(100, 0, shifted_pa);
    append_video_access_unit(shifted, 1000);
    demuxer.push(shifted.data(), shifted.size());
    demuxer.flush();

    const auto selected_video = std::find_if(
        sink.access_units.begin() + static_cast<std::ptrdiff_t>(initial_video_index + 1),
        sink.access_units.end(), [](const auto& unit) {
            return unit.codec == aribtlv::Codec::Hevc;
        });
    check(selected_video != sink.access_units.end() &&
              selected_video->track_id == video_track &&
              selected_video->pts.value == 180000 &&
              selected_video->random_access && selected_video->discontinuity,
          "video track selection reset the timeline or did not resume at a discontinuous RAP");
    check(aribtlv::hasDiscontinuityReason(
              selected_video->discontinuity_reasons,
              aribtlv::DiscontinuityReason::TrackSelection),
          "track selection did not retain its controlled discontinuity reason");
}



} // namespace

int main() {
    test_split_at_every_boundary();
    test_one_byte_input();
    test_garbage_recovery();
    test_service_selection_and_reset();
    test_incomplete_flush();
    test_mode_60_and_resource_limit();
    test_signalling_fragmentation_aggregation_and_m2();
    test_global_packet_state_budget();
    test_track_discovery_and_deduplication();
    test_track_discovery_pq_signal();
    test_truncated_subtitle_reference_start_time_is_rejected();
    test_service_selection_clears_layout_state();
    test_application_and_data_transmission_signalling();
    test_mpt_snapshot_removes_missing_service_state();
    test_mh_ait_snapshot_completion_empty_and_reposition();
    test_service_state_reset_notifications();
    test_independent_m2_sdt_and_tot();
    test_mh_eit_program_events();
    test_mh_eit_hdr_programme_icon();
    test_emt_stream_events();
    test_viewer_participation_notifications();
    test_dynamic_audio_layout_metadata();
    test_authenticated_mmtp_payload_bounds();
    test_codec_output_and_timeline();
    test_recording_scanner_uses_demux_metadata_and_bounds_time();
    test_timestamp_overflow_rejection();
    test_track_selection_clears_incomplete_media();
    test_fragmented_signalling_restart_offset();
    test_reposition_preserves_timeline_and_absolute_offsets();
    test_track_selection_preserves_timeline_and_waits_for_rap();
    std::cout << "all tests passed\n";
    return 0;
}
