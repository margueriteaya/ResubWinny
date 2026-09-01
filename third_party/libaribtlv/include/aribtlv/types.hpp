#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace aribtlv {

enum class Codec { Hevc, AacLatm, Ttml };
enum class TrackKind { Video, Audio, Subtitle };

enum class DiscontinuityReason : std::uint32_t {
    None = 0,
    SourceDamage = 1U << 0U,
    TrackSelection = 1U << 1U,
    Reposition = 1U << 2U,
    TimelineNormalization = 1U << 3U,
};

constexpr DiscontinuityReason operator|(const DiscontinuityReason left,
                                        const DiscontinuityReason right) noexcept {
    return static_cast<DiscontinuityReason>(
        static_cast<std::uint32_t>(left) | static_cast<std::uint32_t>(right));
}

constexpr DiscontinuityReason& operator|=(DiscontinuityReason& left,
                                          const DiscontinuityReason right) noexcept {
    left = left | right;
    return left;
}

constexpr bool hasDiscontinuityReason(const DiscontinuityReason value,
                                      const DiscontinuityReason reason) noexcept {
    return (static_cast<std::uint32_t>(value) & static_cast<std::uint32_t>(reason)) != 0;
}

enum class AudioChannelLayout {
    Unknown,
    Mono,
    DualMono,
    Stereo,
    Channels2_1,
    Channels3_0,
    Channels2_2,
    Channels4_0,
    Channels5_0,
    Channels5_1,
    Channels3_3_1,
    Channels6_1,
    Channels7_1,
    Channels10_2,
    Channels22_2,
};

constexpr std::uint32_t audio_channel_count(const AudioChannelLayout layout) noexcept {
    switch (layout) {
    case AudioChannelLayout::Mono: return 1;
    case AudioChannelLayout::DualMono:
    case AudioChannelLayout::Stereo: return 2;
    case AudioChannelLayout::Channels2_1:
    case AudioChannelLayout::Channels3_0: return 3;
    case AudioChannelLayout::Channels2_2:
    case AudioChannelLayout::Channels4_0: return 4;
    case AudioChannelLayout::Channels5_0: return 5;
    case AudioChannelLayout::Channels5_1: return 6;
    case AudioChannelLayout::Channels3_3_1:
    case AudioChannelLayout::Channels6_1: return 7;
    case AudioChannelLayout::Channels7_1: return 8;
    case AudioChannelLayout::Channels10_2: return 12;
    case AudioChannelLayout::Channels22_2: return 24;
    case AudioChannelLayout::Unknown: return 0;
    }
    return 0;
}

struct AudioInfo {
    std::uint8_t stream_content = 0;
    std::uint8_t component_type = 0;
    std::uint16_t component_tag = 0;
    AudioChannelLayout channel_layout = AudioChannelLayout::Unknown;
    std::uint8_t stream_type = 0;
    std::uint8_t simulcast_group_tag = 0;
    bool es_multi_lingual = false;
    bool main_component = false;
    std::uint8_t quality_indicator = 0;
    std::uint8_t sampling_rate_code = 0;
    std::uint32_t sample_rate = 0;
    std::string secondary_language;
    bool operator==(const AudioInfo&) const = default;
};

// Colour-related programme signalling from ARIB STD-B60 descriptors.  These
// are descriptor values, not a complete CICP tuple for coded video samples.
struct VideoInfo {
    std::optional<std::uint8_t> hdr_wcg_idc;
    std::optional<std::uint8_t> video_transfer_characteristics;
    bool operator==(const VideoInfo&) const = default;
};

struct Timestamp {
    std::int64_t value = 0;
    std::uint32_t timescale = 1;
    bool operator==(const Timestamp&) const = default;
};

// A point which maps the normalized media timeline to the absolute broadcast
// clock. broadcast_time uses the NTP epoch; consumers may project any media
// position from this anchor without tying clock progression to demux speed.
struct BroadcastClock {
    Timestamp media_time;
    Timestamp broadcast_time;
    std::uint64_t input_offset = 0;
    bool discontinuity = false;
};

struct IpDataFlow {
    std::uint32_t context_id = 0;
    std::uint8_t sequence_number = 0;
    std::uint8_t ip_version = 6;
    std::array<std::uint8_t, 16> source_address{};
    std::array<std::uint8_t, 16> destination_address{};
    std::uint8_t next_header = 0;
    std::uint16_t source_port = 0;
    std::uint16_t destination_port = 0;
    std::uint64_t input_offset = 0;
    bool operator==(const IpDataFlow&) const = default;
};

struct TransportNtpClock {
    std::uint8_t ip_version = 6;
    std::array<std::uint8_t, 16> source_address{};
    std::array<std::uint8_t, 16> destination_address{};
    std::uint16_t source_port = 0;
    std::uint16_t destination_port = 0;
    std::uint8_t leap_indicator = 0;
    std::uint8_t version = 0;
    std::uint8_t mode = 0;
    std::uint8_t stratum = 0;
    std::int8_t poll = 0;
    std::int8_t precision = 0;
    std::uint32_t root_delay = 0;
    std::uint32_t root_dispersion = 0;
    std::uint32_t reference_identification = 0;
    std::uint64_t reference_timestamp = 0;
    std::uint64_t origin_timestamp = 0;
    std::uint64_t receive_timestamp = 0;
    std::uint64_t transmit_timestamp = 0;
    // Expanded NTP-era time in microseconds from the NTP epoch.
    Timestamp transmit_time;
    std::uint64_t input_offset = 0;
    bool operator==(const TransportNtpClock&) const = default;
};

struct TlvDescriptor {
    std::uint8_t tag = 0;
    std::vector<std::uint8_t> payload;
    std::uint16_t section_offset = 0;
    bool operator==(const TlvDescriptor&) const = default;
};

struct TlvNetworkStream {
    std::uint16_t tlv_stream_id = 0;
    std::uint16_t original_network_id = 0;
    std::vector<TlvDescriptor> descriptors;
    bool operator==(const TlvNetworkStream&) const = default;
};

struct TlvNetworkInformation {
    std::uint8_t table_id = 0;
    std::uint16_t network_id = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint8_t last_section_number = 0;
    std::vector<TlvDescriptor> network_descriptors;
    std::vector<TlvNetworkStream> streams;
    std::uint64_t input_offset = 0;
    bool operator==(const TlvNetworkInformation&) const = default;
};

struct AddressMapService {
    std::uint16_t service_id = 0;
    std::uint8_t ip_version = 0;
    std::array<std::uint8_t, 16> source_address{};
    std::uint8_t source_prefix_length = 0;
    std::array<std::uint8_t, 16> destination_address{};
    std::uint8_t destination_prefix_length = 0;
    std::vector<std::uint8_t> private_data;
    bool operator==(const AddressMapService&) const = default;
};

struct AddressMap {
    std::uint8_t table_id = 0xfe;
    std::uint16_t table_id_extension = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint8_t last_section_number = 0;
    std::vector<AddressMapService> services;
    std::uint64_t input_offset = 0;
    bool operator==(const AddressMap&) const = default;
};

struct RawSignallingTable {
    std::uint8_t tlv_packet_type = 0xfe;
    std::uint8_t table_id = 0;
    std::uint16_t table_id_extension = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::vector<std::uint8_t> data;
    std::uint64_t input_offset = 0;
};

enum class DescriptorScope : std::uint8_t { Network, TlvStream };

struct UnknownDescriptor {
    std::uint8_t table_id = 0;
    std::uint8_t tag = 0;
    DescriptorScope scope = DescriptorScope::Network;
    std::optional<std::uint16_t> tlv_stream_id;
    std::optional<std::uint16_t> original_network_id;
    std::uint16_t section_offset = 0;
    std::vector<std::uint8_t> payload;
    std::uint64_t input_offset = 0;
};

struct SubtitleInfo {
    std::uint8_t tag = 0;
    std::uint8_t info_version = 0;
    std::uint8_t type = 0;
    std::uint8_t format = 0;
    std::uint8_t operation_mode = 0;
    std::uint8_t timing_mode = 0;
    std::uint8_t display_mode = 0;
    std::uint8_t resolution = 0;
    std::uint8_t compression_type = 0;
    std::optional<std::uint32_t> start_mpu_sequence_number;
    // ARIB STD-B60 reference_start_time in unsigned 64-bit NTP format.
    std::optional<std::uint64_t> reference_start_ntp;
    // ARIB STD-B60 reference_start_time_leap_indicator.
    std::uint8_t reference_start_time_leap_indicator = 0;
};

struct ServiceInfo {
    std::uint32_t context_id = 0;
    std::vector<std::uint8_t> package_id;
};

struct ApplicationServiceInfo {
    std::uint32_t context_id = 0;
    std::uint8_t application_format = 0;
    std::uint8_t document_resolution = 0;
    bool default_ait = false;
    bool has_data_transmission_messages = false;
    std::optional<std::uint16_t> ait_packet_id;
    std::optional<std::uint16_t> data_transmission_packet_id;
    struct EventMessageLocation {
        std::uint8_t event_message_tag = 0;
        std::optional<std::uint16_t> packet_id;
    };
    std::vector<EventMessageLocation> event_message_locations;
};

struct MpuPresentationRegion {
    std::uint32_t mpu_sequence_number = 0;
    std::uint8_t layout_number = 0;
    std::uint8_t region_number = 0;
    bool operator==(const MpuPresentationRegion&) const = default;
};

struct LayoutRegion {
    std::uint8_t region_number = 0;
    std::uint8_t left_top_pos_x = 0;
    std::uint8_t left_top_pos_y = 0;
    std::uint8_t right_down_pos_x = 0;
    std::uint8_t right_down_pos_y = 0;
    std::uint8_t layer_order = 0;
    bool operator==(const LayoutRegion&) const = default;
};

struct LayoutDevice {
    std::uint8_t layout_number = 0;
    std::uint8_t device_id = 0;
    std::vector<LayoutRegion> regions;
    bool operator==(const LayoutDevice&) const = default;
};

struct LayoutConfiguration {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t version = 0;
    std::vector<LayoutDevice> devices;
    // ARIB STD-B60 Background_Color_Descriptor, encoded as 0xRRGGBB.
    std::optional<std::uint32_t> background_color_rgb;
    std::uint64_t input_offset = 0;
    bool operator==(const LayoutConfiguration&) const = default;
};

struct DataAssetInfo {
    std::uint32_t context_id = 0;
    std::uint16_t packet_id = 0;
    std::vector<std::uint8_t> asset_id;
    std::string asset_type;
    std::uint16_t component_tag = 0;
    std::vector<MpuPresentationRegion> presentation_regions;
};

// ARIB STD-B60 Asset Group Descriptor. An asset may belong to more than one
// group, notably when low-layer audio is shared by multiple high-layer assets.
struct AssetGroupInfo {
    std::uint8_t group_identification = 0;
    std::uint8_t selection_level = 0;
    bool operator==(const AssetGroupInfo&) const = default;
};

struct DataUnit {
    std::uint32_t context_id = 0;
    std::uint16_t packet_id = 0;
    std::vector<std::uint8_t> asset_id;
    std::string asset_type;
    std::uint16_t component_tag = 0;
    std::uint32_t mpu_sequence_number = 0;
    std::uint32_t item_id = 0;
    std::optional<std::uint32_t> download_id;
    std::optional<std::uint32_t> item_fragment_number;
    std::optional<std::uint32_t> last_item_fragment_number;
    std::vector<std::uint8_t> data;
    std::uint64_t input_offset = 0;
    bool discontinuity = false;
};

struct SignallingMessage {
    std::uint32_t context_id = 0;
    std::uint16_t packet_id = 0;
    std::uint16_t message_id = 0;
    std::vector<std::uint8_t> data;
    std::uint64_t input_offset = 0;
};

struct ExtendedEventItem {
    std::string description;
    std::string value;
    bool operator==(const ExtendedEventItem&) const = default;
};

struct ContentGenre {
    std::uint8_t level1 = 0;
    std::uint8_t level2 = 0;
    std::uint8_t user1 = 0;
    std::uint8_t user2 = 0;
    bool operator==(const ContentGenre&) const = default;
};

struct ParentalRating {
    std::string country_code;
    std::uint8_t rating = 0;
    bool operator==(const ParentalRating&) const = default;
};

struct EventAudioComponent {
    AudioInfo audio;
    std::string language;
    std::string text;
    bool operator==(const EventAudioComponent&) const = default;
};

struct SeriesInfo {
    std::uint16_t series_id = 0;
    std::uint8_t repeat_label = 0;
    std::uint8_t program_pattern = 0;
    std::optional<std::uint16_t> expire_date_mjd;
    std::uint16_t episode_number = 0;
    std::uint16_t last_episode_number = 0;
    std::string name;
    bool operator==(const SeriesInfo&) const = default;
};

struct EventInfo {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t table_id = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::uint16_t service_id = 0;
    std::uint16_t tlv_stream_id = 0;
    std::uint16_t original_network_id = 0;
    std::uint16_t event_id = 0;
    std::optional<std::int64_t> start_time_unix_milliseconds;
    std::optional<std::uint32_t> duration_seconds;
    std::uint8_t running_status = 0;
    bool free_ca_mode = false;
    std::string language;
    std::string title;
    // The ARIB HDR programme icon was present in the structured short-event
    // title field. This is programme metadata, not a claim about HEVC pixels.
    bool hdr_programme_icon = false;
    std::string description;
    std::string extended_description;
    std::vector<ExtendedEventItem> extended_items;
    std::vector<ContentGenre> genres;
    std::vector<ParentalRating> parental_ratings;
    std::vector<EventAudioComponent> audio_components;
    std::optional<SeriesInfo> series;
    std::uint64_t input_offset = 0;
};

struct ServiceDescriptionInfo {
    std::uint16_t service_id = 0;
    std::uint8_t eit_user_defined_flags = 0;
    bool eit_schedule = false;
    bool eit_present_following = false;
    std::uint8_t running_status = 0;
    bool free_ca_mode = false;
    std::uint8_t service_type = 0;
    std::string provider_name;
    std::string service_name;
    bool operator==(const ServiceDescriptionInfo&) const = default;
};

struct MhSdtSnapshot {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t table_id = 0;
    std::uint16_t tlv_stream_id = 0;
    std::uint16_t original_network_id = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint64_t input_offset = 0;
    std::vector<ServiceDescriptionInfo> services;
};

struct LocalTimeOffsetInfo {
    std::string country_code;
    std::uint8_t country_region_id = 0;
    bool polarity = false;
    std::int32_t offset_minutes = 0;
    std::optional<std::int64_t> change_time_unix_milliseconds;
    std::int32_t next_offset_minutes = 0;
};

struct MhTotInfo {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::int64_t time_unix_milliseconds = 0;
    std::vector<LocalTimeOffsetInfo> local_time_offsets;
    std::uint64_t input_offset = 0;
};

// An ARIB STD-B60 general event message carried by an EMT (table_id 0xA6).
// time_value is the descriptor's raw 64-bit NTP/NPT field.  For time_mode 0
// it is reserved and must not be interpreted as an ignition time.
struct StreamEvent {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t event_message_tag = 0;
    std::uint8_t data_event_id = 0;
    std::uint16_t message_group_id = 0;
    std::uint8_t message_version = 0;
    bool current_next = false;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::uint8_t time_mode = 0;
    std::uint64_t time_value = 0;
    std::optional<std::uint64_t> utc_reference;
    std::optional<std::uint64_t> npt_reference;
    std::uint8_t message_type = 0;
    // B60 carries the application message ID in the high octet and its version
    // in the low octet. Preserve the combined descriptor value for diagnostics.
    std::uint16_t raw_message_id = 0;
    std::uint8_t message_id = 0;
    std::vector<std::uint8_t> private_data;
    std::uint64_t input_offset = 0;
};

// ARIB TR-B39 viewer-participation corner notification carried by a
// descriptor-less EMT (data_event_id 0xF, event_msg_group_id 0xF00).  This is
// a receiver-level notification and must not be injected into the application
// as a general event message.
struct ViewerParticipationNotification {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t event_message_tag = 0xff;
    std::uint8_t data_event_id = 0x0f;
    std::uint16_t message_group_id = 0x0f00;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::uint64_t input_offset = 0;
};

struct ApplicationInfo {
    struct Profile {
        std::uint16_t application_profile = 0;
        std::uint8_t version_major = 0;
        std::uint8_t version_minor = 0;
        std::uint8_t version_micro = 0;

        bool operator==(const Profile&) const = default;
    };

    struct Transport {
        std::uint16_t protocol_id = 0;
        std::uint8_t label = 0;
        std::vector<std::string> urls;
        bool operator==(const Transport&) const = default;
    };

    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint16_t application_type = 0;
    std::uint16_t organization_id = 0;
    std::uint32_t application_id = 0;
    std::uint8_t control_code = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    bool application_descriptor_present = false;
    std::vector<Profile> profiles;
    bool service_bound = false;
    std::uint8_t visibility = 0x03;
    bool present_application_priority = false;
    std::uint8_t application_priority = 0;
    std::vector<std::uint8_t> transport_protocol_labels;
    std::vector<Transport> transports;
    std::string entry_path;
    std::vector<std::string> transport_urls;
    std::uint64_t input_offset = 0;
};

struct DataTransmissionTable {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t table_id = 0;
    std::uint8_t session_id = 0;
    std::uint8_t version = 0;
    bool current_next = true;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::vector<std::uint8_t> data;
    std::uint64_t input_offset = 0;
};

struct DataDirectoryFile {
    std::uint16_t node_tag = 0;
    std::string name;
};

struct DataDirectoryNode {
    std::uint16_t node_tag = 0;
    std::uint8_t version = 0;
    std::string path;
    std::vector<DataDirectoryFile> files;
};

struct DataDirectoryTable {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t session_id = 0;
    std::uint8_t version = 0;
    bool current_next = true;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::string base_path;
    std::vector<DataDirectoryNode> directories;
    std::uint64_t input_offset = 0;
};

struct DataAssetItem {
    std::uint16_t node_tag = 0;
    std::optional<std::uint32_t> item_id;
    std::optional<std::uint32_t> size;
    std::optional<std::uint8_t> version;
    std::optional<std::uint32_t> checksum;
    std::vector<std::uint8_t> info;
};

struct DataAssetMpu {
    std::uint32_t sequence_number = 0;
    std::uint32_t size = 0;
    bool index_item = false;
    std::optional<std::uint32_t> index_item_id;
    std::uint8_t index_item_compression_type = 0;
    std::vector<DataAssetItem> items;
    std::vector<std::uint8_t> info;
};

struct DataAssetManagementTable {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint8_t session_id = 0;
    std::uint8_t version = 0;
    bool current_next = true;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::uint32_t transaction_id = 0;
    std::uint16_t component_tag = 0;
    std::uint32_t download_id = 0;
    std::vector<DataAssetMpu> mpus;
    std::vector<std::uint8_t> component_info;
    std::uint64_t input_offset = 0;
};

struct TrackInfo {
    std::uint64_t track_id = 0;
    std::uint32_t context_id = 0;
    std::uint16_t packet_id = 0;
    std::vector<std::uint8_t> asset_id;
    TrackKind kind = TrackKind::Video;
    Codec codec = Codec::Hevc;
    std::string language;
    std::uint16_t component_tag = 0;
    std::uint32_t timescale = 1;
    std::optional<VideoInfo> video;
    std::optional<AudioInfo> audio;
    std::optional<SubtitleInfo> subtitle;
    std::vector<AssetGroupInfo> asset_groups;
    std::vector<MpuPresentationRegion> presentation_regions;
};

// A complete, validated MPT version.  Consumers should treat this as the
// authoritative service inventory; the item callbacks on Sink are retained as
// a compatibility view derived from committing this snapshot.
struct MptSnapshot {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::vector<std::uint8_t> package_id;
    std::uint8_t version = 0;
    std::uint8_t mode = 0;
    std::uint64_t input_offset = 0;
    std::vector<ApplicationServiceInfo> application_services;
    std::vector<TrackInfo> tracks;
    std::vector<DataAssetInfo> data_assets;
};

// A complete MH-AIT sub-table.  An empty applications vector is meaningful:
// it retires every application from the preceding version of this sub-table.
struct MhAitSnapshot {
    std::uint32_t context_id = 0;
    std::uint16_t source_packet_id = 0;
    std::uint16_t application_type = 0;
    std::uint8_t version = 0;
    bool current_next = false;
    std::uint64_t input_offset = 0;
    std::vector<ApplicationInfo> applications;
};

enum class ServiceStateResetReason { FullReset, ServiceSelection };

struct ServiceStateReset {
    std::optional<std::uint32_t> context_id;
    ServiceStateResetReason reason = ServiceStateResetReason::FullReset;
};

struct SubtitleResource {
    std::uint8_t subsample_number = 0;
    std::uint8_t data_type = 0;
    std::vector<std::uint8_t> data;
};

struct AccessUnit {
    std::uint64_t track_id = 0;
    Codec codec = Codec::Hevc;
    std::uint16_t component_tag = 0;
    std::optional<std::uint8_t> subtitle_timing_mode;
    std::optional<std::uint8_t> subtitle_operation_mode;
    std::optional<std::uint8_t> subtitle_display_mode;
    std::optional<std::uint8_t> subtitle_compression_type;
    std::vector<std::uint8_t> data;
    std::vector<SubtitleResource> subtitle_resources;
    Timestamp pts;
    Timestamp dts;
    std::optional<Timestamp> source_ntp;
    std::optional<std::uint32_t> mpu_sequence_number;
    // Media-timeline position corresponding to SubtitleInfo::reference_start_ntp.
    std::optional<Timestamp> subtitle_reference_start_pts;
    std::uint64_t restart_offset = 0;
    std::uint64_t input_offset = 0;
    bool random_access = false;
    bool discontinuity = false;
    // Distinguishes damaged source media from discontinuities intentionally
    // introduced by track selection or input repositioning.
    DiscontinuityReason discontinuity_reasons = DiscontinuityReason::None;
};

// A damaged media interval begins after the last emitted access unit and ends
// at a decoder recovery point. Video recovery points are random-access units;
// audio and subtitle recovery points are the first subsequent access unit.
struct DamageSpan {
    std::uint64_t track_id = 0;
    TrackKind kind = TrackKind::Video;
    Codec codec = Codec::Hevc;
    std::optional<Timestamp> start_time;
    Timestamp end_time;
    std::optional<Timestamp> recovery_time;
    std::uint64_t start_input_offset = 0;
    std::uint64_t end_input_offset = 0;
    std::uint64_t recovery_input_offset = 0;
    std::uint64_t recovery_restart_offset = 0;
    DiscontinuityReason reasons = DiscontinuityReason::SourceDamage;
    bool recovered = false;
    bool recovery_random_access = false;
};

enum class ErrorCode {
    MalformedInput,
    UnsupportedFeature,
    Discontinuity,
    ResourceLimit,
};

struct Error {
    ErrorCode code = ErrorCode::MalformedInput;
    std::uint64_t input_offset = 0;
    bool recoverable = true;
    std::string message;
};

struct Limits {
    std::size_t max_tlv_payload = 65535;
    std::size_t max_resync_buffer = 1024 * 1024;
    std::size_t max_signalling_message = 1024 * 1024;
    std::size_t max_access_unit = 16 * 1024 * 1024;
    std::size_t max_ttml_sample = 4 * 1024 * 1024;
    std::size_t max_contexts = 64;
    std::size_t max_packet_states = 256;
    bool collect_application_resources = true;
    std::size_t max_application_pending_units = 4096;
    std::size_t max_application_pending_bytes = 64 * 1024 * 1024;
    std::size_t max_application_resource = 16 * 1024 * 1024;
    std::size_t max_application_resources = 4096;
};

} // namespace aribtlv
