#include "mmtp_parser.hpp"

#include <utility>

#include "byte_reader.hpp"

namespace aribtlv::detail {
namespace {

bool descriptor_length(ByteReader& reader, const std::uint16_t tag,
                       std::uint32_t& length) {
    if (tag <= 0x3fff || (tag >= 0x8000 && tag <= 0xefff)) {
        std::uint8_t value = 0;
        if (!reader.read_u8(value)) return false;
        length = value;
        return true;
    }
    if (tag <= 0x6fff || tag >= 0xf000) {
        std::uint16_t value = 0;
        if (!reader.read_u16(value)) return false;
        length = value;
        return true;
    }
    return reader.read_u32(length);
}

} // namespace

bool MmtpParser::parse_tables(const std::uint8_t* data, const std::size_t size,
                              const std::uint16_t packet_id,
                              const std::uint64_t input_offset) {
    ByteReader tables(data, size);
    while (tables.remaining() != 0) {
        std::uint8_t table_id = 0;
        if (!tables.peek_u8(table_id)) return false;
        if (table_id == 0x20 || table_id == 0x80 || table_id == 0x81) {
            if (tables.remaining() < 4) return false;
            const auto table_size = 4 + static_cast<std::size_t>(
                read_be16(tables.current() + 2));
            const std::uint8_t* table = nullptr;
            if (!tables.read_view(table_size, table)) return false;
            const bool valid = table_id == 0x20
                ? parse_mpt(table, table_size, packet_id, input_offset)
                : table_id == 0x80
                    ? parse_package_list(table, table_size, input_offset)
                    : parse_lct(table, table_size, packet_id, input_offset);
            if (!valid) return false;
        } else if (table_id >= 0x81) {
            if (tables.remaining() < 3) return false;
            const auto section_size = 3 + static_cast<std::size_t>(
                read_be16(tables.current() + 1) & 0x0fffU);
            const std::uint8_t* section = nullptr;
            if (!tables.read_view(section_size, section)) return false;
            if (table_id >= 0x8b && table_id <= 0x9b &&
                !parse_mh_eit(section, section_size, packet_id, input_offset)) {
                return false;
            }
            if (table_id == 0x9c &&
                !parse_mh_ait(section, section_size, packet_id, input_offset)) {
                return false;
            }
            if ((table_id == 0x9f || table_id == 0xa0) &&
                !parse_mh_sdt(section, section_size, packet_id, input_offset)) {
                return false;
            }
            if (table_id == 0xa1 &&
                !parse_mh_tot(section, section_size, packet_id, input_offset)) {
                return false;
            }
            if (table_id == 0xa6 &&
                !parse_emt(section, section_size, packet_id, input_offset)) {
                return false;
            }
        } else {
            if (tables.remaining() < 4) return false;
            const auto table_size = 4 + static_cast<std::size_t>(
                read_be16(tables.current() + 2));
            if (!tables.skip(table_size)) return false;
        }
    }
    return true;
}

bool MmtpParser::parse_lct(const std::uint8_t* data, const std::size_t size,
                           const std::uint16_t packet_id,
                           const std::uint64_t input_offset) {
    if (size < 5 || data[0] != 0x81) return false;
    const auto declared_size = static_cast<std::size_t>(read_be16(data + 2));
    if (declared_size != size - 4) return false;

    LayoutConfiguration configuration;
    configuration.context_id = context_id_;
    configuration.source_packet_id = packet_id;
    configuration.version = data[1];
    configuration.input_offset = input_offset;

    ByteReader body(data + 4, declared_size);
    std::uint8_t device_count = 0;
    if (!body.read_u8(device_count)) return false;
    configuration.devices.reserve(device_count);
    for (std::uint16_t device_index = 0; device_index < device_count; ++device_index) {
        LayoutDevice device;
        std::uint8_t region_count = 0;
        if (!body.read_u8(device.layout_number) || !body.read_u8(device.device_id) ||
            !body.read_u8(region_count)) {
            return false;
        }
        device.regions.reserve(region_count);
        for (std::uint16_t region_index = 0; region_index < region_count; ++region_index) {
            LayoutRegion region;
            if (!body.read_u8(region.region_number) ||
                !body.read_u8(region.left_top_pos_x) ||
                !body.read_u8(region.left_top_pos_y) ||
                !body.read_u8(region.right_down_pos_x) ||
                !body.read_u8(region.right_down_pos_y) ||
                !body.read_u8(region.layer_order)) {
                return false;
            }
            device.regions.push_back(region);
        }
        configuration.devices.push_back(std::move(device));
    }

    while (body.remaining() != 0) {
        std::uint16_t tag = 0;
        std::uint32_t length = 0;
        if (!body.read_u16(tag) || !descriptor_length(body, tag, length) ||
            length > body.remaining()) {
            return false;
        }
        const std::uint8_t* payload = nullptr;
        if (!body.read_view(length, payload)) return false;
        if (tag == 0x8002) {
            if (length != 3) return false;
            configuration.background_color_rgb =
                (static_cast<std::uint32_t>(payload[0]) << 16U) |
                (static_cast<std::uint32_t>(payload[1]) << 8U) |
                static_cast<std::uint32_t>(payload[2]);
        }
    }

    on_layout_(std::move(configuration));
    return true;
}

bool MmtpParser::parse_emt(const std::uint8_t* data, const std::size_t size,
                           const std::uint16_t packet_id,
                           const std::uint64_t input_offset) {
    if (size < 12 || data[0] != 0xa6) return false;
    if (!committed_mpt_raw_.empty() &&
        event_message_tags_.find(packet_id) == event_message_tags_.end()) {
        return true;
    }
    const auto section_length = static_cast<std::size_t>(read_be16(data + 1) & 0x0fffU);
    if (section_length + 3 != size || section_length < 9) return false;

    const auto section_end = size - 4;
    const auto data_event_and_group = read_be16(data + 3);
    const auto data_event_id = static_cast<std::uint8_t>(data_event_and_group >> 12U);
    const auto table_group_id = static_cast<std::uint16_t>(data_event_and_group & 0x0fffU);
    const bool current_next = (data[5] & 0x01U) != 0;
    const auto tag = event_message_tags_.find(packet_id);
    const auto event_message_tag = tag == event_message_tags_.end()
        ? std::uint8_t{0} : tag->second;

    struct DescriptorView {
        std::uint16_t tag = 0;
        const std::uint8_t* payload = nullptr;
        std::size_t length = 0;
    };
    std::vector<DescriptorView> descriptor_views;
    ByteReader descriptors(data + 8, section_end - 8);
    while (descriptors.remaining() != 0) {
        std::uint16_t descriptor_tag = 0;
        std::uint32_t descriptor_size = 0;
        if (!descriptors.read_u16(descriptor_tag) ||
            !descriptor_length(descriptors, descriptor_tag, descriptor_size) ||
            descriptor_size > descriptors.remaining()) {
            return false;
        }
        const std::uint8_t* payload = nullptr;
        if (!descriptors.read_view(descriptor_size, payload)) return false;
        descriptor_views.push_back(
            DescriptorView{descriptor_tag, payload, static_cast<std::size_t>(descriptor_size)});
    }

    // TR-B39 6.2.5: the viewer-participation corner notification is the one
    // descriptor-less EMT.  It is consumed by the receiver UI, not dispatched
    // into the ARIB-HTML5 application as a general event message.
    if (section_length == 9 && data_event_id == 0x0f && table_group_id == 0x0f00 &&
        data[6] == 0 && data[7] == 0 && descriptor_views.empty()) {
        ViewerParticipationNotification notification;
        notification.context_id = context_id_;
        notification.source_packet_id = packet_id;
        notification.event_message_tag = tag == event_message_tags_.end()
            ? std::uint8_t{0xff} : tag->second;
        notification.data_event_id = data_event_id;
        notification.message_group_id = table_group_id;
        notification.version = static_cast<std::uint8_t>((data[5] >> 1U) & 0x1fU);
        notification.current_next = current_next;
        notification.section_number = data[6];
        notification.last_section_number = data[7];
        notification.input_offset = input_offset;
        on_viewer_participation_(std::move(notification));
        return true;
    }

    std::optional<std::uint64_t> utc_reference;
    std::optional<std::uint64_t> npt_reference;
    for (const auto& descriptor : descriptor_views) {
        if (descriptor.tag == 0x8021 && descriptor.length >= 17) {
            utc_reference = read_be64(descriptor.payload);
            npt_reference = read_be64(descriptor.payload + 8);
        }
    }

    for (const auto& descriptor : descriptor_views) {
        if (descriptor.tag != 0xf003) continue;
        if (descriptor.length < 14) return false;
        StreamEvent event;
        event.context_id = context_id_;
        event.source_packet_id = packet_id;
        event.event_message_tag = event_message_tag;
        event.data_event_id = data_event_id;
        event.message_group_id = static_cast<std::uint16_t>(
            read_be16(descriptor.payload) >> 4U);
        if (event.message_group_id != table_group_id) return false;
        event.current_next = current_next;
        event.section_number = data[6];
        event.last_section_number = data[7];
        event.time_mode = descriptor.payload[2];
        event.time_value = read_be64(descriptor.payload + 3);
        event.utc_reference = utc_reference;
        event.npt_reference = npt_reference;
        event.message_type = descriptor.payload[11];
        event.raw_message_id = read_be16(descriptor.payload + 12);
        event.message_id = static_cast<std::uint8_t>(event.raw_message_id >> 8U);
        event.message_version = static_cast<std::uint8_t>(event.raw_message_id);
        event.private_data.assign(descriptor.payload + 14,
                                  descriptor.payload + descriptor.length);
        event.input_offset = input_offset;
        on_stream_event_(std::move(event));
    }
    return true;
}

} // namespace aribtlv::detail
