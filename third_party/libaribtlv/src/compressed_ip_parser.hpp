#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <unordered_map>

#include "mmtp_parser.hpp"
#include "tlv_parser.hpp"
#include "tlv_transport_parser.hpp"

namespace aribtlv::detail {

class CompressedIpParser {
public:
    using ServiceCallback = std::function<void(ServiceInfo)>;
    using TrackCallback = std::function<std::uint64_t(TrackInfo)>;
    using AccessUnitCallback = std::function<void(TimedAccessUnit)>;
    using ApplicationServiceCallback = std::function<void(ApplicationServiceInfo)>;
    using LayoutCallback = std::function<void(LayoutConfiguration)>;
    using DataAssetCallback = std::function<void(DataAssetInfo)>;
    using DataUnitCallback = std::function<void(DataUnit)>;
    using SignallingCallback = std::function<void(SignallingMessage)>;
    using EventCallback = std::function<void(EventInfo)>;
    using MhSdtCallback = std::function<void(MhSdtSnapshot)>;
    using MhTotCallback = std::function<void(MhTotInfo)>;
    using StreamEventCallback = std::function<void(StreamEvent)>;
    using ViewerParticipationCallback =
        std::function<void(ViewerParticipationNotification)>;
    using ApplicationCallback = std::function<void(ApplicationInfo)>;
    using MptSnapshotCallback = std::function<void(MptSnapshot)>;
    using MhAitSnapshotCallback = std::function<void(MhAitSnapshot)>;
    using DataTransmissionCallback = std::function<void(DataTransmissionTable)>;
    using DataDirectoryCallback = std::function<void(DataDirectoryTable)>;
    using DataAssetManagementCallback = std::function<void(DataAssetManagementTable)>;
    using FlowCallback = std::function<void(IpDataFlow)>;
    using NtpCallback = std::function<void(TransportNtpClock)>;
    using NitCallback = std::function<void(TlvNetworkInformation)>;
    using AddressMapCallback = std::function<void(AddressMap)>;
    using RawTableCallback = std::function<void(RawSignallingTable)>;
    using UnknownDescriptorCallback = std::function<void(UnknownDescriptor)>;

    CompressedIpParser(const Limits&, ServiceCallback, TrackCallback,
                       AccessUnitCallback, ApplicationServiceCallback,
                       LayoutCallback, DataAssetCallback, DataUnitCallback, SignallingCallback, EventCallback,
                       MhSdtCallback, MhTotCallback,
                       StreamEventCallback, ViewerParticipationCallback,
                       ApplicationCallback, MptSnapshotCallback, MhAitSnapshotCallback,
                       DataTransmissionCallback, DataDirectoryCallback,
                       DataAssetManagementCallback, FlowCallback, NtpCallback,
                       NitCallback, AddressMapCallback, RawTableCallback,
                       UnknownDescriptorCallback, ErrorCallback);

    void consume(const TlvPacketView&);
    void flush();
    void reset();
    void select_service(std::optional<std::uint32_t> context_id);

private:
    MmtpParser* context(std::uint32_t context_id, std::uint64_t input_offset);
    void parse_compressed(const TlvPacketView&);

    Limits limits_;
    ServiceCallback on_service_;
    TrackCallback on_track_;
    AccessUnitCallback on_access_unit_;
    ApplicationServiceCallback on_application_service_;
    LayoutCallback on_layout_;
    DataAssetCallback on_data_asset_;
    DataUnitCallback on_data_unit_;
    SignallingCallback on_signalling_;
    EventCallback on_event_;
    MhSdtCallback on_mh_sdt_;
    MhTotCallback on_mh_tot_;
    StreamEventCallback on_stream_event_;
    ViewerParticipationCallback on_viewer_participation_;
    ApplicationCallback on_application_;
    MptSnapshotCallback on_mpt_snapshot_;
    MhAitSnapshotCallback on_mh_ait_snapshot_;
    DataTransmissionCallback on_data_transmission_;
    DataDirectoryCallback on_data_directory_;
    DataAssetManagementCallback on_data_asset_management_;
    ErrorCallback on_error_;
    TlvTransportParser transport_;
    std::optional<std::uint32_t> selected_service_;
    std::size_t active_packet_states_ = 0;
    std::unordered_map<std::uint32_t, std::unique_ptr<MmtpParser>> contexts_;
};

} // namespace aribtlv::detail
