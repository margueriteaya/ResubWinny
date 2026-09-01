#include <aribtlv/demuxer.hpp>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <string>
#include <utility>
#include <vector>

namespace {

struct Sink final : aribtlv::Sink {
    std::vector<aribtlv::ServiceInfo> services;
    std::vector<aribtlv::IpDataFlow> flows;
    std::vector<aribtlv::TransportNtpClock> ntp;
    std::vector<aribtlv::BroadcastClock> clocks;
    std::vector<aribtlv::TlvNetworkInformation> nit;
    std::vector<aribtlv::AddressMap> address_maps;
    std::vector<aribtlv::RawSignallingTable> raw_tables;
    std::vector<aribtlv::UnknownDescriptor> unknown_descriptors;
    std::vector<aribtlv::Error> errors;
    std::vector<std::string> order;

    void onService(const aribtlv::ServiceInfo& value) override { services.push_back(value); }
    void onTrack(const aribtlv::TrackInfo&) override {}
    void onAccessUnit(aribtlv::AccessUnit&&) override {}
    void onError(const aribtlv::Error& value) override { errors.push_back(value); }
    void onIpDataFlow(const aribtlv::IpDataFlow& value) override { flows.push_back(value); }
    void onTransportNtpClock(const aribtlv::TransportNtpClock& value) override {
        ntp.push_back(value);
    }
    void onBroadcastClock(const aribtlv::BroadcastClock& value) override {
        clocks.push_back(value);
    }
    void onTlvNetworkInformation(const aribtlv::TlvNetworkInformation& value) override {
        order.emplace_back("nit");
        nit.push_back(value);
    }
    void onAddressMap(const aribtlv::AddressMap& value) override {
        order.emplace_back("amt");
        address_maps.push_back(value);
    }
    void onRawSignallingTable(aribtlv::RawSignallingTable&& value) override {
        order.emplace_back("raw");
        raw_tables.push_back(std::move(value));
    }
    void onUnknownDescriptor(aribtlv::UnknownDescriptor&& value) override {
        order.emplace_back("unknown");
        unknown_descriptors.push_back(std::move(value));
    }
};

[[noreturn]] void fail(const std::string& message) {
    std::cerr << "FAIL: " << message << '\n';
    std::exit(1);
}

void check(const bool condition, const std::string& message) {
    if (!condition) fail(message);
}

void append16(std::vector<std::uint8_t>& data, const std::uint16_t value) {
    data.push_back(static_cast<std::uint8_t>(value >> 8U));
    data.push_back(static_cast<std::uint8_t>(value));
}

void append32(std::vector<std::uint8_t>& data, const std::uint32_t value) {
    data.push_back(static_cast<std::uint8_t>(value >> 24U));
    data.push_back(static_cast<std::uint8_t>(value >> 16U));
    data.push_back(static_cast<std::uint8_t>(value >> 8U));
    data.push_back(static_cast<std::uint8_t>(value));
}

std::uint32_t crc32_mpeg(const std::vector<std::uint8_t>& data) {
    std::uint32_t crc = 0xffffffffU;
    for (const auto byte : data) {
        crc ^= static_cast<std::uint32_t>(byte) << 24U;
        for (unsigned bit = 0; bit < 8; ++bit) {
            crc = (crc & 0x80000000U) != 0
                ? (crc << 1U) ^ 0x04c11db7U
                : crc << 1U;
        }
    }
    return crc;
}

std::vector<std::uint8_t> tlv(const std::uint8_t type,
                              const std::vector<std::uint8_t>& payload) {
    std::vector<std::uint8_t> result{0x7f, type};
    append16(result, static_cast<std::uint16_t>(payload.size()));
    result.insert(result.end(), payload.begin(), payload.end());
    return result;
}

std::vector<std::uint8_t> mmtp_signalling() {
    return {
        0x00, 0x02, 0x80, 0x00, 0, 0, 0, 0,
        0, 0, 0, 1, 0, 0, 0, 0,
    };
}

std::vector<std::uint8_t> compressed(const std::uint16_t cid,
                                     const std::uint8_t mode,
                                     const std::uint16_t source_port = 50000,
                                     const std::uint16_t destination_port = 51216,
                                     const std::uint8_t address_marker = 1) {
    std::vector<std::uint8_t> result{
        static_cast<std::uint8_t>((cid << 4U) >> 8U),
        static_cast<std::uint8_t>(cid << 4U), mode,
    };
    if (mode == 0x60) {
        result.insert(result.end(), {0x60, 0, 0, 0, 17, 32});
        result.insert(result.end(), 16, 0);
        result.back() = address_marker;
        result.insert(result.end(), 16, 0);
        result.back() = static_cast<std::uint8_t>(address_marker + 1);
        append16(result, source_port);
        append16(result, destination_port);
    } else if (mode == 0x20) {
        result.insert(result.end(), 20, 0);
    } else if (mode == 0x21) {
        result.insert(result.end(), 2, 0);
    }
    const auto mmtp = mmtp_signalling();
    result.insert(result.end(), mmtp.begin(), mmtp.end());
    return result;
}

std::vector<std::uint8_t> ipv6_ntp() {
    std::vector<std::uint8_t> data{0x60, 0, 0, 0, 0, 56, 17, 32};
    data.insert(data.end(), 16, 0);
    data.back() = 2;
    data.insert(data.end(), 16, 0);
    data[data.size() - 2] = 1;
    data.back() = 1;
    append16(data, 456);
    append16(data, 123);
    append16(data, 56);
    append16(data, 0);
    data.insert(data.end(), {
        0x25, 2, 6, 0xfa,
        0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0,
    });
    for (unsigned index = 0; index < 3; ++index) {
        append32(data, 0);
        append32(data, 0);
    }
    append32(data, 0xa5622500U);
    append32(data, 0x80000000U);
    return data;
}

std::vector<std::uint8_t> extended_section(
    const std::uint8_t table_id, const std::uint16_t extension,
    const std::uint8_t version, const std::uint8_t section_number,
    const std::uint8_t last_section_number, std::vector<std::uint8_t> body) {
    std::vector<std::uint8_t> section{table_id, 0xf0, 0};
    append16(section, extension);
    section.push_back(static_cast<std::uint8_t>(0xc1U | ((version & 0x1fU) << 1U)));
    section.push_back(section_number);
    section.push_back(last_section_number);
    section.insert(section.end(), body.begin(), body.end());
    const auto length = section.size() - 3 + 4;
    section[1] = static_cast<std::uint8_t>(0xf0U | (length >> 8U));
    section[2] = static_cast<std::uint8_t>(length);
    append32(section, crc32_mpeg(section));
    check(crc32_mpeg(section) == 0, "fixture CRC did not close to zero");
    return section;
}

std::vector<std::uint8_t> nit_section(const std::uint8_t version = 3,
                                      const std::uint8_t section_number = 0,
                                      const std::uint8_t last_section_number = 0) {
    std::vector<std::uint8_t> body;
    append16(body, 0xf003);
    body.insert(body.end(), {0xe1, 1, 0xaa}); // deliberately unknown descriptor
    std::vector<std::uint8_t> streams;
    append16(streams, static_cast<std::uint16_t>(0x100 + section_number));
    append16(streams, 0x000b);
    append16(streams, 0xf003);
    streams.insert(streams.end(), {0x41, 1, static_cast<std::uint8_t>(0x60 + section_number)});
    append16(body, static_cast<std::uint16_t>(0xf000U | streams.size()));
    body.insert(body.end(), streams.begin(), streams.end());
    return extended_section(0x40, 0x000b, version, section_number,
                            last_section_number, std::move(body));
}

std::vector<std::uint8_t> address_map_section(const std::uint8_t version = 2) {
    std::vector<std::uint8_t> body;
    append16(body, 0x007f); // one service id plus six reserved one-bits
    append16(body, 0x0065);
    append16(body, 0xfc24); // IPv6, reserved, 36-byte loop
    body.insert(body.end(), 16, 0);
    body.back() = 2;
    body.push_back(128);
    body.insert(body.end(), 16, 0);
    body[body.size() - 2] = 0xff;
    body.back() = 0x3e;
    body.push_back(128);
    body.insert(body.end(), {0xde, 0xad});
    return extended_section(0xfe, 0, version, 0, 0, std::move(body));
}

void push_and_flush(aribtlv::Demuxer& demuxer, const std::vector<std::uint8_t>& data) {
    demuxer.push(data.data(), data.size());
    demuxer.flush();
}

void test_dispatch_flow_and_reset() {
    Sink sink;
    aribtlv::Demuxer demuxer(sink);
    auto stream = tlv(0x01, {0x45});
    for (const auto& packet : {
             tlv(0x03, compressed(1, 0x20)),
             tlv(0x03, compressed(1, 0x21)),
             tlv(0x03, compressed(1, 0x60)),
             tlv(0x03, compressed(1, 0x61)),
             tlv(0xff, {0xff, 0xff}),
         }) {
        stream.insert(stream.end(), packet.begin(), packet.end());
    }
    push_and_flush(demuxer, stream);
    check(sink.services.size() == 1 && sink.services.front().context_id == 1,
          "compressed modes did not share their CID or direct 0x61 failed");
    check(sink.flows.size() == 1 && sink.flows.front().source_port == 50000 &&
              sink.flows.front().destination_port == 51216,
          "mode 0x60 did not publish the observed IPv6/UDP flow");
    check(std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
        return error.message.find("IPv4 TLV packet is recognized") != std::string::npos;
    }), "IPv4 packet was not observable");
    check(std::none_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
        return error.message.find("null") != std::string::npos;
    }), "null TLV packet produced an error");

    const auto same = tlv(0x03, compressed(1, 0x60));
    push_and_flush(demuxer, same);
    check(sink.flows.size() == 1, "unchanged flow was emitted again");
    const auto changed = tlv(0x03, compressed(1, 0x60, 40000, 40001, 9));
    push_and_flush(demuxer, changed);
    check(sink.flows.size() == 2 && sink.services.size() == 1,
          "changed non-standard flow did not notify while MMTP continued");
    demuxer.reset();
    push_and_flush(demuxer, same);
    check(sink.flows.size() == 3, "reset retained compressed-IP flow state");
}

void test_ntp_and_reset() {
    Sink sink;
    aribtlv::Demuxer demuxer(sink);
    const auto packet = tlv(0x02, ipv6_ntp());
    push_and_flush(demuxer, packet);
    check(sink.ntp.size() == 1, "IPv6/UDP NTP did not publish an event");
    check(sink.ntp.front().version == 4 && sink.ntp.front().mode == 5 &&
              sink.ntp.front().destination_port == 123,
          "NTP header fields were not decoded");
    check(sink.ntp.front().transmit_timestamp == 0xa562250080000000ULL,
          "NTP transmit timestamp was not preserved");
    check(!sink.clocks.empty() && sink.clocks.front().broadcast_time ==
              sink.ntp.front().transmit_time,
          "transport NTP did not seed the broadcast clock");
    demuxer.reset();
    check(!demuxer.broadcastClock().has_value(), "reset retained transport clock state");

    push_and_flush(demuxer, packet);
    demuxer.reposition(aribtlv::RepositionOptions{0, true});
    check(!demuxer.broadcastClock().has_value(),
          "timeline-preserving reposition retained a transport-derived clock");
}

void test_resynchronization_clears_transport_state() {
    Sink sink;
    aribtlv::Demuxer demuxer(sink);
    const auto flow = tlv(0x03, compressed(1, 0x60));
    const auto null_packet = tlv(0xff, {});

    auto initial = flow;
    initial.insert(initial.end(), null_packet.begin(), null_packet.end());
    demuxer.push(initial.data(), initial.size());
    check(sink.flows.size() == 1, "initial flow was not observed");

    std::vector<std::uint8_t> damaged{0x00, 0x01};
    damaged.insert(damaged.end(), flow.begin(), flow.end());
    damaged.insert(damaged.end(), null_packet.begin(), null_packet.end());
    demuxer.push(damaged.data(), damaged.size());
    check(sink.flows.size() == 2,
          "TLV resynchronization retained and deduplicated the old CID flow");

    const auto ntp_packet = tlv(0x02, ipv6_ntp());
    auto clock_stream = ntp_packet;
    clock_stream.insert(clock_stream.end(), null_packet.begin(), null_packet.end());
    demuxer.push(clock_stream.data(), clock_stream.size());
    check(demuxer.broadcastClock().has_value(), "transport NTP did not seed a clock");
    std::vector<std::uint8_t> clock_damage{0x00};
    clock_damage.insert(clock_damage.end(), null_packet.begin(), null_packet.end());
    clock_damage.insert(clock_damage.end(), null_packet.begin(), null_packet.end());
    demuxer.push(clock_damage.data(), clock_damage.size());
    check(!demuxer.broadcastClock().has_value(),
          "TLV resynchronization retained a transport-derived clock");

    const auto section = nit_section();
    const auto split = section.size() / 2;
    auto first = tlv(0xfe, {section.begin(),
                            section.begin() + static_cast<std::ptrdiff_t>(split)});
    first.insert(first.end(), null_packet.begin(), null_packet.end());
    demuxer.push(first.data(), first.size());
    const auto before = sink.nit.size();
    std::vector<std::uint8_t> section_damage{0x00};
    const auto tail = tlv(0xfe, {
        section.begin() + static_cast<std::ptrdiff_t>(split), section.end()});
    section_damage.insert(section_damage.end(), tail.begin(), tail.end());
    section_damage.insert(section_damage.end(), null_packet.begin(), null_packet.end());
    demuxer.push(section_damage.data(), section_damage.size());
    check(sink.nit.size() == before,
          "TLV-SI section was assembled across a resynchronization boundary");
    auto complete = tlv(0xfe, section);
    complete.insert(complete.end(), null_packet.begin(), null_packet.end());
    demuxer.push(complete.data(), complete.size());
    check(sink.nit.size() == before + 1,
          "TLV-SI parser did not recover after resynchronization");
}

void test_tlv_si_and_deduplication() {
    Sink sink;
    aribtlv::Demuxer demuxer(sink);
    const auto nit = nit_section();
    const auto split = nit.size() / 2;
    auto stream = tlv(0xfe, {nit.begin(), nit.begin() + static_cast<std::ptrdiff_t>(split)});
    const auto tail = tlv(0xfe, {nit.begin() + static_cast<std::ptrdiff_t>(split), nit.end()});
    stream.insert(stream.end(), tail.begin(), tail.end());
    push_and_flush(demuxer, stream);
    check(sink.raw_tables.size() == 1 && sink.nit.size() == 1,
          "split TLV-NIT section was not reassembled");
    check(sink.nit.front().network_id == 0x000b && sink.nit.front().streams.size() == 1,
          "TLV-NIT snapshot lost network or stream-loop fields");
    check(sink.unknown_descriptors.size() == 1 &&
              sink.unknown_descriptors.front().tag == 0xe1 &&
              sink.unknown_descriptors.front().section_offset == 10,
          "unknown descriptor was not retained with location");
    check(sink.order.size() >= 3 && sink.order[0] == "raw" &&
              sink.order[1] == "unknown" && sink.order[2] == "nit",
          "raw TLV-SI event was not published before derived events");

    push_and_flush(demuxer, tlv(0xfe, nit));
    check(sink.raw_tables.size() == 2 && sink.nit.size() == 1,
          "duplicate section did not remain raw-observable or repeated typed snapshot");

    const auto first = nit_section(4, 0, 1);
    const auto second = nit_section(4, 1, 1);
    push_and_flush(demuxer, tlv(0xfe, first));
    check(sink.nit.size() == 1, "incomplete version emitted a snapshot");
    push_and_flush(demuxer, tlv(0xfe, second));
    check(sink.nit.size() == 2 && sink.nit.back().streams.size() == 2 &&
              sink.nit.back().version == 4,
          "version switch or multi-section assembly failed");

    push_and_flush(demuxer, tlv(0xfe, address_map_section()));
    check(sink.address_maps.size() == 1 && sink.address_maps.front().services.size() == 1,
          "AMT snapshot was not decoded");
    const auto& service = sink.address_maps.front().services.front();
    check(service.service_id == 0x65 && service.ip_version == 6 &&
              service.source_prefix_length == 128 &&
              service.destination_prefix_length == 128 &&
              service.private_data == std::vector<std::uint8_t>({0xde, 0xad}),
          "AMT IPv6 service/private data was incomplete");
}

void test_bad_crc_and_unknown_table() {
    Sink sink;
    aribtlv::Demuxer demuxer(sink);
    auto bad = nit_section();
    bad.back() ^= 1;
    push_and_flush(demuxer, tlv(0xfe, bad));
    check(sink.raw_tables.empty() && sink.nit.empty(),
          "invalid CRC updated raw or typed TLV-SI state");
    check(std::any_of(sink.errors.begin(), sink.errors.end(), [](const auto& error) {
        return error.message.find("CRC") != std::string::npos;
    }), "invalid CRC was not diagnosed");

    const auto unknown = extended_section(0x7e, 0x1234, 1, 0, 0, {1, 2, 3});
    push_and_flush(demuxer, tlv(0xfe, unknown));
    check(sink.raw_tables.size() == 1 && sink.raw_tables.front().table_id == 0x7e &&
              sink.nit.empty() && sink.address_maps.empty(),
          "unknown valid table was not preserved as raw signalling");
}

} // namespace

int main() {
    test_dispatch_flow_and_reset();
    test_ntp_and_reset();
    test_resynchronization_clears_transport_state();
    test_tlv_si_and_deduplication();
    test_bad_crc_and_unknown_table();
    std::cout << "tlv transport tests passed\n";
    return 0;
}
