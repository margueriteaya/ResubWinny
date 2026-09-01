#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>

#include <aribtlv/types.hpp>
#include <aribtlv/application_resources.hpp>

namespace aribtlv {

struct RepositionOptions {
    std::uint64_t input_offset = 0;
    bool preserve_timeline = true;
};

class Sink {
public:
    virtual ~Sink() = default;
    virtual void onService(const ServiceInfo&) = 0;
    virtual void onTrack(const TrackInfo&) = 0;
    virtual void onAccessUnit(AccessUnit&&) = 0;
    virtual void onError(const Error&) = 0;
    virtual void onDamage(const DamageSpan&) {}
    virtual void onBroadcastClock(const BroadcastClock&) {}
    virtual void onIpDataFlow(const IpDataFlow&) {}
    virtual void onTransportNtpClock(const TransportNtpClock&) {}
    virtual void onTlvNetworkInformation(const TlvNetworkInformation&) {}
    virtual void onAddressMap(const AddressMap&) {}
    virtual void onRawSignallingTable(RawSignallingTable&&) {}
    virtual void onUnknownDescriptor(UnknownDescriptor&&) {}
    virtual void onEventInfo(const EventInfo&) {}
    virtual void onMhSdtSnapshot(const MhSdtSnapshot&) {}
    virtual void onMhTot(const MhTotInfo&) {}
    virtual void onStreamEvent(const StreamEvent&) {}
    virtual void onViewerParticipationNotification(
        const ViewerParticipationNotification&) {}
    virtual void onApplicationService(const ApplicationServiceInfo&) {}
    virtual void onApplicationServiceRemoved(const ApplicationServiceInfo&) {}
    virtual void onLayoutConfiguration(const LayoutConfiguration&) {}
    virtual void onDataAsset(const DataAssetInfo&) {}
    virtual void onDataAssetRemoved(const DataAssetInfo&) {}
    virtual void onDataUnit(DataUnit&&) {}
    virtual void onSignallingMessage(SignallingMessage&&) {}
    virtual void onApplication(const ApplicationInfo&) {}
    virtual void onApplicationRemoved(const ApplicationInfo&) {}
    virtual void onMptSnapshot(const MptSnapshot&) {}
    virtual void onMhAitSnapshot(const MhAitSnapshot&) {}
    virtual void onTrackRemoved(const TrackInfo&) {}
    virtual void onServiceStateReset(const ServiceStateReset&) {}
    virtual void onDataTransmissionTable(DataTransmissionTable&&) {}
    virtual void onDataDirectoryTable(const DataDirectoryTable&) {}
    virtual void onDataAssetManagementTable(const DataAssetManagementTable&) {}
    virtual void onApplicationState(const ApplicationState&) {}
    virtual void onApplicationResource(ApplicationResource&&) {}
    virtual void onApplicationResourceRemoved(const ApplicationResourceRemoval&) {}
    virtual void onApplicationResourcesReset() {}
};

class Demuxer {
public:
    explicit Demuxer(Sink&);
    Demuxer(Sink&, Limits);
    ~Demuxer();

    Demuxer(Demuxer&&) noexcept;
    Demuxer& operator=(Demuxer&&) noexcept;
    Demuxer(const Demuxer&) = delete;
    Demuxer& operator=(const Demuxer&) = delete;

    void push(const std::uint8_t* data, std::size_t size);
    void flush();
    void reset();
    void reposition(RepositionOptions);
    void selectService(std::optional<std::uint32_t> context_id);
    void selectTrack(TrackKind kind, std::optional<std::uint64_t> track_id);
    void setSubtitlePassthroughEnabled(bool enabled);
    std::optional<BroadcastClock> broadcastClock() const;

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace aribtlv
