#include <aribtlv/demuxer.hpp>

#include <array>
#include <limits>
#include <string>
#include <unordered_map>
#include <utility>

#include "compressed_ip_parser.hpp"

namespace aribtlv {

namespace {

class ForwardApplicationResourceSink final : public ApplicationResourceSink {
public:
    explicit ForwardApplicationResourceSink(Sink& sink) : sink_(sink) {}

    void onApplicationState(const ApplicationState& state) override {
        sink_.onApplicationState(state);
    }
    void onApplicationResource(ApplicationResource&& resource) override {
        sink_.onApplicationResource(std::move(resource));
    }
    void onApplicationResourceRemoved(
        const ApplicationResourceRemoval& removal) override {
        sink_.onApplicationResourceRemoved(removal);
    }
    void onApplicationResourcesReset() override {
        sink_.onApplicationResourcesReset();
    }
    void onApplicationResourceError(const Error& error) override {
        sink_.onError(error);
    }

private:
    Sink& sink_;
};

} // namespace

class Demuxer::Impl {
public:
    Impl(Sink& sink, Limits limits)
        : sink_(sink), limits_(std::move(limits)),
          application_sink_(sink_), application_resources_(application_sink_, limits_),
          ip_(limits_,
              [this](ServiceInfo info) { service(std::move(info)); },
              [this](TrackInfo info) { return track(std::move(info)); },
              [this](detail::TimedAccessUnit unit) { access_unit(std::move(unit)); },
              [this](ApplicationServiceInfo info) { application_service(std::move(info)); },
              [this](LayoutConfiguration info) { layout(std::move(info)); },
              [this](DataAssetInfo info) { data_asset(std::move(info)); },
              [this](DataUnit unit) { data_unit(std::move(unit)); },
              [this](SignallingMessage message) { signalling(std::move(message)); },
              [this](EventInfo info) { event_info(std::move(info)); },
              [this](MhSdtSnapshot snapshot) { mh_sdt(std::move(snapshot)); },
              [this](MhTotInfo info) { mh_tot(std::move(info)); },
              [this](StreamEvent event) { stream_event(std::move(event)); },
              [this](ViewerParticipationNotification notification) {
                  viewer_participation(std::move(notification));
              },
              [this](ApplicationInfo info) { application(std::move(info)); },
              [this](MptSnapshot snapshot) { mpt_snapshot(std::move(snapshot)); },
              [this](MhAitSnapshot snapshot) { mh_ait_snapshot(std::move(snapshot)); },
              [this](DataTransmissionTable table) { data_transmission(std::move(table)); },
              [this](DataDirectoryTable table) { data_directory(std::move(table)); },
              [this](DataAssetManagementTable table) { data_asset_management(std::move(table)); },
              [this](IpDataFlow flow) { sink_.onIpDataFlow(flow); },
              [this](TransportNtpClock clock) { transport_ntp(std::move(clock)); },
              [this](TlvNetworkInformation info) { sink_.onTlvNetworkInformation(info); },
              [this](AddressMap map) { sink_.onAddressMap(map); },
              [this](RawSignallingTable table) {
                  sink_.onRawSignallingTable(std::move(table));
              },
              [this](UnknownDescriptor descriptor) {
                  sink_.onUnknownDescriptor(std::move(descriptor));
              },
              [this](const ErrorCode code, const std::uint64_t offset,
                     const bool recoverable, std::string message) {
                  error(code, offset, recoverable, std::move(message));
              }),
          tlv_(limits_,
               [this](const detail::TlvPacketView& packet) { ip_.consume(packet); },
               [this](const std::uint64_t) { transport_discontinuity(); },
               [this](const ErrorCode code, const std::uint64_t offset,
                      const bool recoverable, std::string message) {
                   error(code, offset, recoverable, std::move(message));
               }) {}

    void push(const std::uint8_t* data, const std::size_t size) {
        if (data == nullptr && size != 0) {
            error(ErrorCode::MalformedInput, 0, false,
                  "Demuxer::push received a null pointer with non-zero size");
            return;
        }
        tlv_.push(data, size);
        if (size > std::numeric_limits<std::uint64_t>::max() - input_end_offset_) {
            input_end_offset_ = std::numeric_limits<std::uint64_t>::max();
        } else {
            input_end_offset_ += size;
        }
    }

    void flush() {
        tlv_.flush();
        ip_.flush();
        finish_damage_spans();
    }

    void reset() {
        sink_.onServiceStateReset(
            ServiceStateReset{selected_service_, ServiceStateResetReason::FullReset});
        tlv_.reset();
        ip_.reset();
        if (limits_.collect_application_resources) application_resources_.reset();
        clear_service_state();
        error_counts_.clear();
        clear_timeline_state();
        clear_damage_state();
        reposition_epoch_ = 0;
        emitted_reposition_epochs_.clear();
        input_end_offset_ = 0;
        arm_track_selections(0);
    }

    void reposition(const RepositionOptions options) {
        tlv_.reset(options.input_offset);
        ip_.reset();
        transport_ntp_.reset();
        if (broadcast_clock_from_transport_) broadcast_clock_.reset();
        broadcast_clock_from_transport_ = false;
        if (limits_.collect_application_resources) {
            application_resources_.dropTransientKeepActive();
        }
        if (!options.preserve_timeline) origin_.reset();
        if (!options.preserve_timeline) broadcast_clock_.reset();
        last_clock_mpu_sequence_.reset();
        clear_damage_state();
        input_end_offset_ = options.input_offset;
        arm_track_selections(options.input_offset);
        ++reposition_epoch_;
        if (reposition_epoch_ == 0) ++reposition_epoch_;
    }

    void select_service(std::optional<std::uint32_t> context_id) {
        sink_.onServiceStateReset(
            ServiceStateReset{selected_service_, ServiceStateResetReason::ServiceSelection});
        selected_service_ = context_id;
        ip_.select_service(context_id);
        if (limits_.collect_application_resources) application_resources_.reset();
        clear_service_state();
        clear_timeline_state();
        clear_damage_state();
    }

    void select_track(const TrackKind kind, std::optional<std::uint64_t> track_id) {
        const auto index = static_cast<std::size_t>(kind);
        if (selected_tracks_[index] == track_id) return;
        selected_tracks_[index] = track_id;
        selection_boundaries_[index] = input_end_offset_;
        selection_pending_[index] = track_id.has_value();
        selection_wait_for_rap_[index] =
            kind == TrackKind::Video && track_id.has_value();
        if (kind == TrackKind::Video) last_clock_mpu_sequence_.reset();
    }

    void set_subtitle_passthrough_enabled(const bool enabled) {
        subtitle_passthrough_enabled_ = enabled;
    }

    std::optional<BroadcastClock> broadcast_clock() const { return broadcast_clock_; }

private:
    void transport_ntp(TransportNtpClock clock) {
        transport_ntp_ = clock;
        sink_.onTransportNtpClock(clock);
        if (!broadcast_clock_.has_value()) {
            broadcast_clock_ = BroadcastClock{
                Timestamp{0, 1000000}, clock.transmit_time,
                clock.input_offset, false,
            };
            broadcast_clock_from_transport_ = true;
            sink_.onBroadcastClock(*broadcast_clock_);
        }
    }

    void transport_discontinuity() {
        ip_.reset();
        transport_ntp_.reset();
        if (broadcast_clock_from_transport_) broadcast_clock_.reset();
        broadcast_clock_from_transport_ = false;
    }

    void clear_service_state() {
        services_.clear();
        current_tracks_.clear();
        application_services_.clear();
        layouts_.clear();
        data_assets_.clear();
        events_.clear();
        mh_sdt_snapshots_.clear();
        mh_tot_.clear();
        stream_events_.clear();
        viewer_participation_notifications_.clear();
        applications_.clear();
        data_transmission_tables_.clear();
        data_directory_versions_.clear();
        data_asset_management_versions_.clear();
        mpt_snapshots_.clear();
        mh_ait_snapshots_.clear();
    }

    void clear_timeline_state() {
        origin_.reset();
        broadcast_clock_.reset();
        transport_ntp_.reset();
        broadcast_clock_from_transport_ = false;
        last_clock_mpu_sequence_.reset();
    }

    void clear_damage_state() {
        pending_damage_.clear();
        last_access_units_.clear();
    }

    void arm_track_selections(const std::uint64_t boundary) {
        for (std::size_t index = 0; index < selected_tracks_.size(); ++index) {
            selection_boundaries_[index] = boundary;
            selection_pending_[index] = selected_tracks_[index].has_value();
            selection_wait_for_rap_[index] =
                index == static_cast<std::size_t>(TrackKind::Video) &&
                selected_tracks_[index].has_value();
        }
    }

    static bool same_track(const TrackInfo& left, const TrackInfo& right) {
        const auto same_audio = [](const std::optional<AudioInfo>& first,
                                   const std::optional<AudioInfo>& second) {
            if (first.has_value() != second.has_value()) return false;
            if (!first.has_value()) return true;
            return first->stream_content == second->stream_content &&
                   first->component_type == second->component_type &&
                   first->component_tag == second->component_tag &&
                   first->channel_layout == second->channel_layout &&
                   first->stream_type == second->stream_type &&
                   first->simulcast_group_tag == second->simulcast_group_tag &&
                   first->es_multi_lingual == second->es_multi_lingual &&
                   first->main_component == second->main_component &&
                   first->quality_indicator == second->quality_indicator &&
                   first->sampling_rate_code == second->sampling_rate_code &&
                   first->sample_rate == second->sample_rate &&
                   first->secondary_language == second->secondary_language;
        };
        const auto same_subtitle = [](const std::optional<SubtitleInfo>& first,
                                      const std::optional<SubtitleInfo>& second) {
            if (first.has_value() != second.has_value()) return false;
            if (!first.has_value()) return true;
            return first->tag == second->tag &&
                   first->info_version == second->info_version &&
                   first->type == second->type && first->format == second->format &&
                   first->operation_mode == second->operation_mode &&
                   first->timing_mode == second->timing_mode &&
                   first->display_mode == second->display_mode &&
                   first->resolution == second->resolution &&
                   first->compression_type == second->compression_type &&
                   first->start_mpu_sequence_number == second->start_mpu_sequence_number &&
                   first->reference_start_ntp == second->reference_start_ntp &&
                   first->reference_start_time_leap_indicator ==
                       second->reference_start_time_leap_indicator;
        };
        return left.track_id == right.track_id && left.context_id == right.context_id &&
               left.packet_id == right.packet_id && left.asset_id == right.asset_id &&
               left.kind == right.kind && left.codec == right.codec &&
               left.language == right.language && left.component_tag == right.component_tag &&
               left.timescale == right.timescale && left.video == right.video &&
               same_audio(left.audio, right.audio) &&
               same_subtitle(left.subtitle, right.subtitle) &&
               left.asset_groups == right.asset_groups &&
               left.presentation_regions == right.presentation_regions;
    }

    static bool same_application_service(const ApplicationServiceInfo& left,
                                         const ApplicationServiceInfo& right) {
        if (left.context_id != right.context_id ||
            left.application_format != right.application_format ||
            left.document_resolution != right.document_resolution ||
            left.default_ait != right.default_ait ||
            left.has_data_transmission_messages != right.has_data_transmission_messages ||
            left.ait_packet_id != right.ait_packet_id ||
            left.data_transmission_packet_id != right.data_transmission_packet_id ||
            left.event_message_locations.size() != right.event_message_locations.size()) {
            return false;
        }
        for (std::size_t index = 0; index < left.event_message_locations.size(); ++index) {
            if (left.event_message_locations[index].event_message_tag !=
                    right.event_message_locations[index].event_message_tag ||
                left.event_message_locations[index].packet_id !=
                    right.event_message_locations[index].packet_id) {
                return false;
            }
        }
        return true;
    }

    static bool same_data_asset(const DataAssetInfo& left, const DataAssetInfo& right) {
        return left.context_id == right.context_id && left.packet_id == right.packet_id &&
            left.asset_id == right.asset_id && left.asset_type == right.asset_type &&
            left.component_tag == right.component_tag &&
            left.presentation_regions == right.presentation_regions;
    }

    static bool same_application(const ApplicationInfo& left, const ApplicationInfo& right) {
        return left.context_id == right.context_id &&
            left.source_packet_id == right.source_packet_id &&
            left.application_type == right.application_type &&
            left.organization_id == right.organization_id &&
            left.application_id == right.application_id &&
            left.control_code == right.control_code && left.version == right.version &&
            left.current_next == right.current_next &&
            left.section_number == right.section_number &&
            left.last_section_number == right.last_section_number &&
            left.application_descriptor_present == right.application_descriptor_present &&
            left.profiles == right.profiles && left.service_bound == right.service_bound &&
            left.visibility == right.visibility &&
            left.present_application_priority == right.present_application_priority &&
            left.application_priority == right.application_priority &&
            left.transport_protocol_labels == right.transport_protocol_labels &&
            left.transports == right.transports && left.entry_path == right.entry_path &&
            left.transport_urls == right.transport_urls;
    }

    void service(ServiceInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) {
            return;
        }
        const auto found = services_.find(info.context_id);
        if (found != services_.end() && info.package_id.empty() && !found->second.empty()) {
            return;
        }
        if (found != services_.end() && found->second == info.package_id) {
            return;
        }
        services_[info.context_id] = info.package_id;
        sink_.onService(info);
    }

    std::uint64_t track(TrackInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) {
            return 0;
        }
        std::string identity;
        identity.reserve(6 + info.asset_id.size());
        for (const auto shift : {24U, 16U, 8U, 0U}) {
            identity.push_back(static_cast<char>((info.context_id >> shift) & 0xffU));
        }
        identity.push_back(static_cast<char>(info.packet_id >> 8U));
        identity.push_back(static_cast<char>(info.packet_id & 0xffU));
        identity.append(reinterpret_cast<const char*>(info.asset_id.data()), info.asset_id.size());

        auto id = track_ids_.find(identity);
        if (id == track_ids_.end()) {
            id = track_ids_.emplace(std::move(identity), next_track_id_++).first;
        }
        info.track_id = id->second;
        return info.track_id;
    }

    void mpt_snapshot(MptSnapshot snapshot) {
        if (selected_service_.has_value() && *selected_service_ != snapshot.context_id) return;
        const auto previous = mpt_snapshots_.find(snapshot.context_id);
        bool unchanged = previous != mpt_snapshots_.end() &&
            previous->second.version == snapshot.version &&
            previous->second.mode == snapshot.mode &&
            previous->second.package_id == snapshot.package_id &&
            previous->second.application_services.size() == snapshot.application_services.size() &&
            previous->second.tracks.size() == snapshot.tracks.size() &&
            previous->second.data_assets.size() == snapshot.data_assets.size();
        if (unchanged) {
            for (std::size_t index = 0; index < snapshot.application_services.size(); ++index) {
                if (!same_application_service(previous->second.application_services[index],
                                              snapshot.application_services[index])) {
                    unchanged = false;
                    break;
                }
            }
        }
        if (unchanged) {
            for (std::size_t index = 0; index < snapshot.tracks.size(); ++index) {
                if (!same_track(previous->second.tracks[index], snapshot.tracks[index])) {
                    unchanged = false;
                    break;
                }
            }
        }
        if (unchanged) {
            for (std::size_t index = 0; index < snapshot.data_assets.size(); ++index) {
                if (!same_data_asset(previous->second.data_assets[index],
                                     snapshot.data_assets[index])) {
                    unchanged = false;
                    break;
                }
            }
        }
        if (unchanged) return;

        sink_.onMptSnapshot(snapshot);
        for (auto it = current_tracks_.begin(); it != current_tracks_.end();) {
            if (it->second.context_id != snapshot.context_id) {
                ++it;
                continue;
            }
            const auto found = std::find_if(snapshot.tracks.begin(), snapshot.tracks.end(),
                                            [id = it->first](const auto& value) {
                                                return value.track_id == id;
                                            });
            if (found == snapshot.tracks.end()) {
                sink_.onTrackRemoved(it->second);
                it = current_tracks_.erase(it);
            } else {
                ++it;
            }
        }
        for (const auto& info : snapshot.tracks) {
            const auto found = current_tracks_.find(info.track_id);
            if (found == current_tracks_.end() || !same_track(found->second, info)) {
                current_tracks_[info.track_id] = info;
                sink_.onTrack(info);
            }
        }

        std::vector<ApplicationServiceInfo> old_services;
        for (auto it = application_services_.begin(); it != application_services_.end();) {
            if (it->second.context_id == snapshot.context_id) {
                old_services.push_back(it->second);
                it = application_services_.erase(it);
            } else {
                ++it;
            }
        }
        for (const auto& old : old_services) {
            if (std::none_of(snapshot.application_services.begin(),
                             snapshot.application_services.end(), [&](const auto& value) {
                                 return same_application_service(old, value);
                             })) {
                sink_.onApplicationServiceRemoved(old);
                if (old.ait_packet_id.has_value()) retire_mh_ait_source(
                    snapshot.context_id, *old.ait_packet_id);
            }
        }
        for (std::size_t index = 0; index < snapshot.application_services.size(); ++index) {
            const auto& info = snapshot.application_services[index];
            application_services_[std::to_string(snapshot.context_id) + ':' +
                                  std::to_string(index)] = info;
            if (std::none_of(old_services.begin(), old_services.end(), [&](const auto& value) {
                    return same_application_service(value, info);
                })) sink_.onApplicationService(info);
        }

        std::vector<DataAssetInfo> old_assets;
        for (auto it = data_assets_.begin(); it != data_assets_.end();) {
            if (it->second.context_id == snapshot.context_id) {
                old_assets.push_back(it->second);
                it = data_assets_.erase(it);
            } else {
                ++it;
            }
        }
        for (const auto& old : old_assets) {
            if (std::none_of(snapshot.data_assets.begin(), snapshot.data_assets.end(),
                             [&](const auto& value) { return same_data_asset(old, value); })) {
                sink_.onDataAssetRemoved(old);
            }
        }
        for (std::size_t index = 0; index < snapshot.data_assets.size(); ++index) {
            const auto& info = snapshot.data_assets[index];
            data_assets_[std::to_string(snapshot.context_id) + ':' +
                         std::to_string(index)] = info;
            if (std::none_of(old_assets.begin(), old_assets.end(),
                             [&](const auto& value) { return same_data_asset(value, info); })) {
                sink_.onDataAsset(info);
            }
        }
        mpt_snapshots_[snapshot.context_id] = std::move(snapshot);
    }

    void retire_mh_ait_source(const std::uint32_t context_id,
                              const std::uint16_t source_packet_id) {
        for (auto it = mh_ait_snapshots_.begin(); it != mh_ait_snapshots_.end();) {
            if (it->second.context_id != context_id ||
                it->second.source_packet_id != source_packet_id) {
                ++it;
                continue;
            }
            for (const auto& application_info : it->second.applications) {
                const auto key = std::to_string(application_info.context_id) + ':' +
                    std::to_string(application_info.source_packet_id) + ':' +
                    std::to_string(application_info.application_type) + ':' +
                    std::to_string(application_info.organization_id) + ':' +
                    std::to_string(application_info.application_id);
                applications_.erase(key);
                sink_.onApplicationRemoved(application_info);
            }
            it = mh_ait_snapshots_.erase(it);
        }
    }

    bool accepts_mh_ait(const MhAitSnapshot& snapshot) const {
        const auto mpt = mpt_snapshots_.find(snapshot.context_id);
        if (mpt == mpt_snapshots_.end()) return true;
        return std::any_of(mpt->second.application_services.begin(),
                           mpt->second.application_services.end(), [&](const auto& service) {
                               return service.ait_packet_id == snapshot.source_packet_id;
                           });
    }

    bool accepts_data_transmission(const std::uint32_t context_id,
                                   const std::uint16_t packet_id) const {
        const auto mpt = mpt_snapshots_.find(context_id);
        if (mpt == mpt_snapshots_.end()) return true;
        return std::any_of(mpt->second.application_services.begin(),
                           mpt->second.application_services.end(), [&](const auto& service) {
                               return service.has_data_transmission_messages &&
                                   service.data_transmission_packet_id == packet_id;
                           });
    }

    bool accepts_emt(const std::uint32_t context_id,
                     const std::uint16_t packet_id) const {
        const auto mpt = mpt_snapshots_.find(context_id);
        if (mpt == mpt_snapshots_.end()) return true;
        for (const auto& service : mpt->second.application_services) {
            if (std::any_of(service.event_message_locations.begin(),
                            service.event_message_locations.end(), [&](const auto& location) {
                                return location.packet_id == packet_id;
                            })) return true;
        }
        return false;
    }

    void mh_ait_snapshot(MhAitSnapshot snapshot) {
        if (selected_service_.has_value() && *selected_service_ != snapshot.context_id) return;
        if (!accepts_mh_ait(snapshot)) return;
        sink_.onMhAitSnapshot(snapshot);
        if (!snapshot.current_next) return;
        const auto snapshot_key = std::to_string(snapshot.context_id) + ':' +
            std::to_string(snapshot.source_packet_id) + ':' +
            std::to_string(snapshot.application_type);
        std::vector<ApplicationInfo> old_applications;
        const auto previous = mh_ait_snapshots_.find(snapshot_key);
        if (previous != mh_ait_snapshots_.end()) old_applications = previous->second.applications;
        for (const auto& old : old_applications) {
            if (std::none_of(snapshot.applications.begin(), snapshot.applications.end(),
                             [&](const auto& value) {
                                 return value.organization_id == old.organization_id &&
                                     value.application_id == old.application_id;
                             })) {
                const auto key = std::to_string(old.context_id) + ':' +
                    std::to_string(old.source_packet_id) + ':' +
                    std::to_string(old.application_type) + ':' +
                    std::to_string(old.organization_id) + ':' +
                    std::to_string(old.application_id);
                applications_.erase(key);
                sink_.onApplicationRemoved(old);
            }
        }
        for (const auto& info : snapshot.applications) application(info);
        mh_ait_snapshots_[snapshot_key] = std::move(snapshot);
    }

    void application_service(ApplicationServiceInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) return;
        const auto key = std::to_string(info.context_id) + ':' +
            std::to_string(info.ait_packet_id.value_or(0xffffU)) + ':' +
            std::to_string(info.data_transmission_packet_id.value_or(0xffffU));
        const auto found = application_services_.find(key);
        if (found != application_services_.end() &&
            found->second.application_format == info.application_format &&
            found->second.document_resolution == info.document_resolution &&
            found->second.default_ait == info.default_ait &&
            found->second.has_data_transmission_messages == info.has_data_transmission_messages) {
            return;
        }
        application_services_[key] = info;
        sink_.onApplicationService(info);
    }

    void data_asset(DataAssetInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) return;
        std::string key = std::to_string(info.context_id) + ':' + std::to_string(info.packet_id) + ':';
        key.append(reinterpret_cast<const char*>(info.asset_id.data()), info.asset_id.size());
        const auto found = data_assets_.find(key);
        if (found != data_assets_.end() && found->second.asset_type == info.asset_type &&
            found->second.component_tag == info.component_tag &&
            found->second.presentation_regions == info.presentation_regions) {
            return;
        }
        data_assets_[key] = info;
        sink_.onDataAsset(info);
    }

    void layout(LayoutConfiguration info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) return;
        const auto key = std::to_string(info.context_id) + ':' +
            std::to_string(info.source_packet_id);
        const auto found = layouts_.find(key);
        if (found != layouts_.end() && found->second.version == info.version &&
            found->second.devices == info.devices &&
            found->second.background_color_rgb == info.background_color_rgb) {
            return;
        }
        layouts_[key] = info;
        sink_.onLayoutConfiguration(info);
    }

    void data_unit(DataUnit unit) {
        if (selected_service_.has_value() && *selected_service_ != unit.context_id) return;
        application_resources_.onDataUnit(unit);
        sink_.onDataUnit(std::move(unit));
    }

    void signalling(SignallingMessage message) {
        if (selected_service_.has_value() && *selected_service_ != message.context_id) return;
        sink_.onSignallingMessage(std::move(message));
    }

    void event_info(EventInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) return;
        const auto key = std::to_string(info.context_id) + ':' +
            std::to_string(info.table_id) + ':' + std::to_string(info.service_id) + ':' +
            std::to_string(info.section_number) + ':' + std::to_string(info.event_id);
        const auto found = events_.find(key);
        if (found != events_.end() && found->second.version == info.version &&
            found->second.start_time_unix_milliseconds == info.start_time_unix_milliseconds &&
            found->second.duration_seconds == info.duration_seconds &&
            found->second.title == info.title && found->second.description == info.description &&
            found->second.extended_description == info.extended_description &&
            found->second.extended_items == info.extended_items &&
            found->second.genres == info.genres &&
            found->second.parental_ratings == info.parental_ratings &&
            found->second.audio_components == info.audio_components &&
            found->second.series == info.series &&
            found->second.hdr_programme_icon == info.hdr_programme_icon) {
            return;
        }
        events_[key] = info;
        sink_.onEventInfo(info);
    }

    void mh_sdt(MhSdtSnapshot snapshot) {
        if (selected_service_.has_value() && *selected_service_ != snapshot.context_id) return;
        if (!snapshot.current_next) return;
        const auto key = std::to_string(snapshot.context_id) + ':' +
            std::to_string(snapshot.source_packet_id) + ':' +
            std::to_string(snapshot.table_id) + ':' +
            std::to_string(snapshot.tlv_stream_id) + ':' +
            std::to_string(snapshot.original_network_id);
        const auto found = mh_sdt_snapshots_.find(key);
        if (found != mh_sdt_snapshots_.end() &&
            found->second.version == snapshot.version &&
            found->second.services == snapshot.services) return;
        mh_sdt_snapshots_[key] = snapshot;
        sink_.onMhSdtSnapshot(snapshot);
    }

    void mh_tot(MhTotInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) return;
        const auto key = std::to_string(info.context_id) + ':' +
            std::to_string(info.source_packet_id);
        const auto found = mh_tot_.find(key);
        if (found != mh_tot_.end() &&
            found->second.time_unix_milliseconds == info.time_unix_milliseconds) return;
        mh_tot_[key] = info;
        sink_.onMhTot(info);
    }

    void stream_event(StreamEvent event) {
        if (selected_service_.has_value() && *selected_service_ != event.context_id) return;
        if (!accepts_emt(event.context_id, event.source_packet_id)) return;
        if (!event.current_next) return;
        const auto key = std::to_string(event.context_id) + ':' +
            std::to_string(event.event_message_tag) + ':' +
            std::to_string(event.data_event_id) + ':' +
            std::to_string(event.message_group_id) + ':' +
            std::to_string(event.raw_message_id);
        const auto found = stream_events_.find(key);
        if (found != stream_events_.end() &&
            found->second.message_version == event.message_version) {
            return;
        }
        stream_events_[key] = event;
        sink_.onStreamEvent(event);
    }

    void viewer_participation(ViewerParticipationNotification notification) {
        if (selected_service_.has_value() &&
            *selected_service_ != notification.context_id) {
            return;
        }
        if (!accepts_emt(notification.context_id, notification.source_packet_id)) return;
        if (!notification.current_next) return;
        const auto key = std::to_string(notification.context_id) + ':' +
            std::to_string(notification.source_packet_id) + ':' +
            std::to_string(notification.event_message_tag) + ':' +
            std::to_string(notification.data_event_id) + ':' +
            std::to_string(notification.message_group_id);
        const auto found = viewer_participation_notifications_.find(key);
        if (found != viewer_participation_notifications_.end() &&
            found->second.version == notification.version) {
            return;
        }
        viewer_participation_notifications_[key] = notification;
        sink_.onViewerParticipationNotification(notification);
    }

    void application(ApplicationInfo info) {
        if (selected_service_.has_value() && *selected_service_ != info.context_id) return;
        const auto key = std::to_string(info.context_id) + ':' +
            std::to_string(info.source_packet_id) + ':' +
            std::to_string(info.application_type) + ':' +
            std::to_string(info.organization_id) + ':' + std::to_string(info.application_id);
        const auto found = applications_.find(key);
        if (found != applications_.end() && same_application(found->second, info)) {
            return;
        }
        applications_[key] = info;
        sink_.onApplication(info);
        application_resources_.onApplication(info);
    }

    void data_transmission(DataTransmissionTable table) {
        if (selected_service_.has_value() && *selected_service_ != table.context_id) return;
        if (!accepts_data_transmission(table.context_id, table.source_packet_id)) return;
        const auto key = std::to_string(table.context_id) + ':' +
            std::to_string(table.table_id) + ':' + std::to_string(table.session_id) + ':' +
            std::to_string(table.section_number);
        const auto found = data_transmission_tables_.find(key);
        if (found != data_transmission_tables_.end() && found->second.version == table.version &&
            found->second.data == table.data) {
            return;
        }
        data_transmission_tables_[key] = table;
        sink_.onDataTransmissionTable(std::move(table));
    }

    void data_directory(DataDirectoryTable table) {
        if (selected_service_.has_value() && *selected_service_ != table.context_id) return;
        const auto key = std::to_string(table.context_id) + ':' +
            std::to_string(table.session_id) + ':' + std::to_string(table.section_number);
        const auto found = data_directory_versions_.find(key);
        if (found != data_directory_versions_.end() && found->second == table.version) return;
        data_directory_versions_[key] = table.version;
        sink_.onDataDirectoryTable(table);
        application_resources_.onDataDirectoryTable(table);
    }

    void data_asset_management(DataAssetManagementTable table) {
        if (selected_service_.has_value() && *selected_service_ != table.context_id) return;
        const auto key = std::to_string(table.context_id) + ':' +
            std::to_string(table.session_id) + ':' + std::to_string(table.section_number);
        const auto found = data_asset_management_versions_.find(key);
        if (found != data_asset_management_versions_.end() && found->second == table.version) return;
        data_asset_management_versions_[key] = table.version;
        sink_.onDataAssetManagementTable(table);
        application_resources_.onDataAssetManagementTable(table);
    }

    static bool ntp_delta_ticks(const std::uint64_t current, const std::uint64_t origin,
                                const std::uint32_t timescale, std::int64_t& output) {
        const bool negative = current < origin;
        const auto difference = negative ? origin - current : current - origin;
        const auto seconds = difference >> 32U;
        const auto fraction = static_cast<std::uint32_t>(difference);
        if (timescale != 0 && seconds >
            static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max()) / timescale) {
            return false;
        }
        const auto whole_ticks = seconds * timescale;
        const auto fractional_ticks =
            (static_cast<std::uint64_t>(fraction) * timescale) >> 32U;
        if (whole_ticks > static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max()) -
                              fractional_ticks) {
            return false;
        }
        const auto magnitude = static_cast<std::int64_t>(whole_ticks + fractional_ticks);
        output = negative ? -magnitude : magnitude;
        return true;
    }

    static bool rescale_offset(const std::int64_t value, const std::uint32_t source_scale,
                               const std::uint32_t target_scale, std::int64_t& output) {
        if (source_scale == 0) return false;
        const bool negative = value < 0;
        const auto magnitude = negative
            ? static_cast<std::uint64_t>(-(value + 1)) + 1U
            : static_cast<std::uint64_t>(value);
        const auto whole = magnitude / source_scale;
        const auto remainder = magnitude % source_scale;
        if (target_scale != 0 && whole >
            static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max()) / target_scale) {
            return false;
        }
        const auto scaled = whole * target_scale + remainder * target_scale / source_scale;
        if (scaled > static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max())) {
            return false;
        }
        output = negative ? -static_cast<std::int64_t>(scaled) : static_cast<std::int64_t>(scaled);
        return true;
    }

    static bool add_checked(const std::int64_t left, const std::int64_t right,
                            std::int64_t& output) {
        if ((right > 0 && left > std::numeric_limits<std::int64_t>::max() - right) ||
            (right < 0 && left < std::numeric_limits<std::int64_t>::min() - right)) {
            return false;
        }
        output = left + right;
        return true;
    }

    static bool subtract_checked(const std::int64_t left, const std::int64_t right,
                                 std::int64_t& output) {
        if ((right > 0 && left < std::numeric_limits<std::int64_t>::min() + right) ||
            (right < 0 && left > std::numeric_limits<std::int64_t>::max() + right)) {
            return false;
        }
        output = left - right;
        return true;
    }

    void access_unit(detail::TimedAccessUnit timed) {
        auto& unit = timed.unit;
        const auto track_info = current_tracks_.find(unit.track_id);
        if (unit.track_id == 0 || track_info == current_tracks_.end()) return;
        const auto kind_index = static_cast<std::size_t>(track_info->second.kind);
        const bool bypass_selection = subtitle_passthrough_enabled_ &&
                                      track_info->second.kind == TrackKind::Subtitle;
        if (!bypass_selection && selected_tracks_[kind_index].has_value() &&
            *selected_tracks_[kind_index] != unit.track_id) {
            return;
        }
        if (!bypass_selection && selection_pending_[kind_index]) {
            if (unit.input_offset < selection_boundaries_[kind_index]) return;
            if (selection_wait_for_rap_[kind_index] && !unit.random_access) return;
            unit.discontinuity = true;
            unit.discontinuity_reasons |= DiscontinuityReason::TrackSelection;
            selection_pending_[kind_index] = false;
            selection_wait_for_rap_[kind_index] = false;
        }
        if (unit.pts.timescale == 0 || unit.dts.timescale != unit.pts.timescale) {
            error(ErrorCode::MalformedInput, unit.input_offset, true,
                  "access unit has an invalid or inconsistent timescale");
            return;
        }
        const auto presentation_offset = unit.pts;
        if (!origin_.has_value()) {
            origin_ = TimelineOrigin{timed.source_ntp_raw, unit.pts.value, unit.pts.timescale};
        }

        std::int64_t ntp_ticks = 0;
        std::int64_t origin_offset = 0;
        if (!ntp_delta_ticks(timed.source_ntp_raw, origin_->ntp, unit.pts.timescale, ntp_ticks) ||
            !rescale_offset(origin_->pts_offset, origin_->timescale,
                            unit.pts.timescale, origin_offset)) {
            error(ErrorCode::Discontinuity, unit.input_offset, true,
                  "timestamp normalization overflowed");
            return;
        }
        std::int64_t normalized_pts = 0;
        std::int64_t normalized_dts = 0;
        std::int64_t base = 0;
        if (!subtract_checked(ntp_ticks, origin_offset, base) ||
            !add_checked(base, unit.pts.value, normalized_pts) ||
            !add_checked(base, unit.dts.value, normalized_dts)) {
            error(ErrorCode::Discontinuity, unit.input_offset, true,
                  "normalized access-unit timestamp is out of range");
            return;
        }
        unit.pts.value = normalized_pts;
        unit.dts.value = normalized_dts;

        if (track_info->second.kind == TrackKind::Video && unit.source_ntp.has_value()) {
            std::int64_t offset_us = 0;
            std::int64_t broadcast_us = 0;
            if (rescale_offset(presentation_offset.value, presentation_offset.timescale,
                               1000000, offset_us) &&
                add_checked(unit.source_ntp->value, offset_us, broadcast_us)) {
                broadcast_clock_ = BroadcastClock{
                    unit.pts,
                    Timestamp{broadcast_us, 1000000},
                    unit.input_offset,
                    unit.discontinuity,
                };
                broadcast_clock_from_transport_ = false;
                if (!last_clock_mpu_sequence_.has_value() ||
                    last_clock_mpu_sequence_ != unit.mpu_sequence_number ||
                    unit.discontinuity) {
                    last_clock_mpu_sequence_ = unit.mpu_sequence_number;
                    sink_.onBroadcastClock(*broadcast_clock_);
                }
            } else {
                error(ErrorCode::Discontinuity, unit.input_offset, true,
                      "broadcast clock mapping overflowed");
            }
        }
        if (track_info->second.subtitle &&
            track_info->second.subtitle->reference_start_ntp) {
            std::int64_t reference_ticks = 0;
            std::int64_t reference_pts = 0;
            if (ntp_delta_ticks(*track_info->second.subtitle->reference_start_ntp,
                                origin_->ntp, unit.pts.timescale, reference_ticks) &&
                subtract_checked(reference_ticks, origin_offset, reference_pts)) {
                unit.subtitle_reference_start_pts =
                    Timestamp{reference_pts, unit.pts.timescale};
            } else {
                unit.discontinuity = true;
                unit.discontinuity_reasons |=
                    DiscontinuityReason::TimelineNormalization;
                error(ErrorCode::Discontinuity, unit.input_offset, true,
                      "subtitle reference timestamp normalization overflowed");
            }
        }
        if (reposition_epoch_ != 0 &&
            emitted_reposition_epochs_[unit.track_id] != reposition_epoch_) {
            unit.discontinuity = true;
            unit.discontinuity_reasons |= DiscontinuityReason::Reposition;
            emitted_reposition_epochs_[unit.track_id] = reposition_epoch_;
        }
        if (unit.discontinuity && unit.discontinuity_reasons == DiscontinuityReason::None) {
            unit.discontinuity_reasons = DiscontinuityReason::SourceDamage;
        }
        observe_damage(unit, track_info->second);
        sink_.onAccessUnit(std::move(unit));
    }

    struct AccessUnitBoundary {
        Timestamp time;
        std::uint64_t input_offset = 0;
    };

    struct PendingDamage {
        TrackKind kind = TrackKind::Video;
        Codec codec = Codec::Hevc;
        std::optional<Timestamp> start_time;
        Timestamp end_time;
        std::uint64_t start_input_offset = 0;
        std::uint64_t end_input_offset = 0;
        DiscontinuityReason reasons = DiscontinuityReason::None;
    };

    void observe_damage(const AccessUnit& unit, const TrackInfo& track) {
        const bool source_damage = hasDiscontinuityReason(
            unit.discontinuity_reasons, DiscontinuityReason::SourceDamage);
        auto pending = pending_damage_.find(unit.track_id);
        if (source_damage && pending == pending_damage_.end()) {
            PendingDamage damage;
            damage.kind = track.kind;
            damage.codec = unit.codec;
            const auto previous = last_access_units_.find(unit.track_id);
            if (previous != last_access_units_.end()) {
                damage.start_time = previous->second.time;
                damage.start_input_offset = previous->second.input_offset;
            } else {
                damage.start_time = unit.pts;
                damage.start_input_offset = unit.input_offset;
            }
            damage.end_time = unit.pts;
            damage.end_input_offset = unit.input_offset;
            damage.reasons = DiscontinuityReason::SourceDamage;
            pending = pending_damage_.emplace(unit.track_id, std::move(damage)).first;
        }

        if (pending != pending_damage_.end()) {
            pending->second.end_time = unit.pts;
            pending->second.end_input_offset = unit.input_offset;
            pending->second.reasons |= unit.discontinuity_reasons;
            const bool recovery = track.kind != TrackKind::Video || unit.random_access;
            if (recovery) {
                sink_.onDamage(DamageSpan{
                    unit.track_id,
                    track.kind,
                    unit.codec,
                    pending->second.start_time,
                    unit.pts,
                    unit.pts,
                    pending->second.start_input_offset,
                    unit.input_offset,
                    unit.input_offset,
                    unit.restart_offset,
                    pending->second.reasons,
                    true,
                    unit.random_access,
                });
                pending_damage_.erase(pending);
            }
        }

        last_access_units_[unit.track_id] = AccessUnitBoundary{unit.pts, unit.input_offset};
    }

    void finish_damage_spans() {
        for (const auto& [track_id, damage] : pending_damage_) {
            sink_.onDamage(DamageSpan{
                track_id,
                damage.kind,
                damage.codec,
                damage.start_time,
                damage.end_time,
                std::nullopt,
                damage.start_input_offset,
                damage.end_input_offset,
                0,
                0,
                damage.reasons,
                false,
                false,
            });
        }
        pending_damage_.clear();
    }

    void error(const ErrorCode code, const std::uint64_t offset, const bool recoverable,
               std::string message) {
        const auto key = std::to_string(static_cast<int>(code)) + ':' + message;
        auto& count = error_counts_[key];
        ++count;
        const bool power_of_two = (count & (count - 1U)) == 0;
        if (power_of_two) {
            sink_.onError(Error{code, offset, recoverable, std::move(message)});
        }
    }

    Sink& sink_;
    Limits limits_;
    ForwardApplicationResourceSink application_sink_;
    ApplicationResourceAssembler application_resources_;
    detail::CompressedIpParser ip_;
    detail::TlvParser tlv_;
    std::optional<std::uint32_t> selected_service_;
    std::array<std::optional<std::uint64_t>, 3> selected_tracks_{};
    std::array<std::uint64_t, 3> selection_boundaries_{};
    std::array<bool, 3> selection_pending_{};
    std::array<bool, 3> selection_wait_for_rap_{};
    bool subtitle_passthrough_enabled_ = false;
    std::uint64_t input_end_offset_ = 0;
    std::unordered_map<std::uint32_t, std::vector<std::uint8_t>> services_;
    std::unordered_map<std::string, std::uint64_t> track_ids_;
    std::unordered_map<std::uint64_t, TrackInfo> current_tracks_;
    std::unordered_map<std::string, ApplicationServiceInfo> application_services_;
    std::unordered_map<std::string, LayoutConfiguration> layouts_;
    std::unordered_map<std::string, DataAssetInfo> data_assets_;
    std::unordered_map<std::string, EventInfo> events_;
    std::unordered_map<std::string, MhSdtSnapshot> mh_sdt_snapshots_;
    std::unordered_map<std::string, MhTotInfo> mh_tot_;
    std::unordered_map<std::string, StreamEvent> stream_events_;
    std::unordered_map<std::string, ViewerParticipationNotification>
        viewer_participation_notifications_;
    std::unordered_map<std::string, ApplicationInfo> applications_;
    std::unordered_map<std::string, DataTransmissionTable> data_transmission_tables_;
    std::unordered_map<std::string, std::uint8_t> data_directory_versions_;
    std::unordered_map<std::string, std::uint8_t> data_asset_management_versions_;
    std::unordered_map<std::uint32_t, MptSnapshot> mpt_snapshots_;
    std::unordered_map<std::string, MhAitSnapshot> mh_ait_snapshots_;
    std::uint64_t next_track_id_ = 1;
    std::unordered_map<std::string, std::uint64_t> error_counts_;
    std::uint64_t reposition_epoch_ = 0;
    std::unordered_map<std::uint64_t, std::uint64_t> emitted_reposition_epochs_;
    std::unordered_map<std::uint64_t, AccessUnitBoundary> last_access_units_;
    std::unordered_map<std::uint64_t, PendingDamage> pending_damage_;
    struct TimelineOrigin {
        std::uint64_t ntp = 0;
        std::int64_t pts_offset = 0;
        std::uint32_t timescale = 1;
    };
    std::optional<TimelineOrigin> origin_;
    std::optional<BroadcastClock> broadcast_clock_;
    std::optional<TransportNtpClock> transport_ntp_;
    bool broadcast_clock_from_transport_ = false;
    std::optional<std::uint32_t> last_clock_mpu_sequence_;
};

Demuxer::Demuxer(Sink& sink) : Demuxer(sink, Limits{}) {}

Demuxer::Demuxer(Sink& sink, Limits limits)
    : impl_(std::make_unique<Impl>(sink, std::move(limits))) {}

Demuxer::~Demuxer() = default;
Demuxer::Demuxer(Demuxer&&) noexcept = default;
Demuxer& Demuxer::operator=(Demuxer&&) noexcept = default;

void Demuxer::push(const std::uint8_t* data, const std::size_t size) {
    impl_->push(data, size);
}

void Demuxer::flush() {
    impl_->flush();
}

void Demuxer::reset() {
    impl_->reset();
}

void Demuxer::reposition(const RepositionOptions options) {
    impl_->reposition(options);
}

void Demuxer::selectService(std::optional<std::uint32_t> context_id) {
    impl_->select_service(context_id);
}

void Demuxer::selectTrack(const TrackKind kind, std::optional<std::uint64_t> track_id) {
    impl_->select_track(kind, track_id);
}

void Demuxer::setSubtitlePassthroughEnabled(const bool enabled) {
    impl_->set_subtitle_passthrough_enabled(enabled);
}

std::optional<BroadcastClock> Demuxer::broadcastClock() const {
    return impl_->broadcast_clock();
}

} // namespace aribtlv
