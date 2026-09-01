#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <tuple>
#include <vector>

#include "parser_common.hpp"
#include "tlv_parser.hpp"

namespace aribtlv::detail {

class TlvTransportParser {
public:
    using FlowCallback = std::function<void(IpDataFlow)>;
    using NtpCallback = std::function<void(TransportNtpClock)>;
    using NitCallback = std::function<void(TlvNetworkInformation)>;
    using AddressMapCallback = std::function<void(AddressMap)>;
    using RawTableCallback = std::function<void(RawSignallingTable)>;
    using UnknownDescriptorCallback = std::function<void(UnknownDescriptor)>;

    TlvTransportParser(FlowCallback, NtpCallback, NitCallback, AddressMapCallback,
                       RawTableCallback, UnknownDescriptorCallback, ErrorCallback);

    void consume_ipv4(const TlvPacketView&);
    void consume_ipv6(const TlvPacketView&);
    bool observe_compressed_flow(const TlvPacketView&, std::uint32_t context_id,
                                 std::uint8_t sequence_number);
    void consume_tlv_si(const TlvPacketView&);
    void consume_null(const TlvPacketView&);
    void reset();
    std::optional<std::uint64_t> latest_ntp() const noexcept { return latest_ntp_; }

private:
    struct NitSection {
        std::uint8_t table_id = 0;
        std::uint16_t network_id = 0;
        std::uint8_t version = 0;
        bool current_next = false;
        std::uint8_t section_number = 0;
        std::uint8_t last_section_number = 0;
        std::vector<TlvDescriptor> network_descriptors;
        std::vector<TlvNetworkStream> streams;
        std::uint64_t input_offset = 0;
    };

    struct AddressMapSection {
        std::uint16_t table_id_extension = 0;
        std::uint8_t version = 0;
        bool current_next = false;
        std::uint8_t section_number = 0;
        std::uint8_t last_section_number = 0;
        std::vector<AddressMapService> services;
        std::uint64_t input_offset = 0;
    };

    template <typename Section>
    struct Assembly {
        std::uint8_t last_section_number = 0;
        std::map<std::uint8_t, Section> sections;
        bool emitted = false;
    };

    using TableKey = std::tuple<std::uint8_t, std::uint16_t, std::uint8_t, bool>;

    void process_sections();
    void process_section(const std::vector<std::uint8_t>&, std::uint64_t input_offset);
    std::optional<NitSection> parse_nit(const std::vector<std::uint8_t>&,
                                        std::uint64_t input_offset);
    std::optional<AddressMapSection> parse_address_map(
        const std::vector<std::uint8_t>&, std::uint64_t input_offset);
    std::optional<std::vector<TlvDescriptor>> parse_descriptors(
        const std::vector<std::uint8_t>&, std::size_t begin, std::size_t length,
        std::uint8_t table_id, DescriptorScope,
        std::optional<std::uint16_t> tlv_stream_id,
        std::optional<std::uint16_t> original_network_id,
        std::uint64_t input_offset);
    void accept_nit(NitSection);
    void accept_address_map(AddressMapSection);
    void report(ErrorCode, std::uint64_t, std::string);

    FlowCallback on_flow_;
    NtpCallback on_ntp_;
    NitCallback on_nit_;
    AddressMapCallback on_address_map_;
    RawTableCallback on_raw_table_;
    UnknownDescriptorCallback on_unknown_descriptor_;
    ErrorCallback on_error_;
    std::map<std::uint32_t, IpDataFlow> flows_;
    std::vector<std::uint8_t> section_buffer_;
    std::vector<std::uint64_t> section_origins_;
    std::map<TableKey, Assembly<NitSection>> nit_assemblies_;
    std::map<TableKey, Assembly<AddressMapSection>> address_map_assemblies_;
    std::optional<std::uint64_t> latest_ntp_;
    std::uint64_t null_packet_count_ = 0;
};

} // namespace aribtlv::detail
