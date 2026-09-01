#include <aribtlv/application_resources.hpp>

#include <algorithm>
#include <condition_variable>
#include <map>
#include <mutex>
#include <optional>
#include <set>
#include <sstream>
#include <tuple>
#include <unordered_map>
#include <utility>

#include <zlib.h>

namespace aribtlv {
namespace {

std::uint16_t be16(const std::uint8_t* data) {
    return static_cast<std::uint16_t>((static_cast<std::uint16_t>(data[0]) << 8U) | data[1]);
}

std::uint32_t be32(const std::uint8_t* data) {
    return (static_cast<std::uint32_t>(data[0]) << 24U) |
           (static_cast<std::uint32_t>(data[1]) << 16U) |
           (static_cast<std::uint32_t>(data[2]) << 8U) | data[3];
}

struct Cursor {
    const std::vector<std::uint8_t>& data;
    std::size_t at = 0;

    bool u8(std::uint8_t& value) {
        if (at >= data.size()) return false;
        value = data[at++];
        return true;
    }
    bool u16(std::uint16_t& value) {
        if (data.size() - at < 2) return false;
        value = be16(data.data() + at);
        at += 2;
        return true;
    }
    bool u32(std::uint32_t& value) {
        if (data.size() - at < 4) return false;
        value = be32(data.data() + at);
        at += 4;
        return true;
    }
    bool text(const std::size_t size, std::string& value) {
        if (size > data.size() - at) return false;
        value.assign(reinterpret_cast<const char*>(data.data() + at), size);
        at += size;
        return true;
    }
    bool skip(const std::size_t size) {
        if (size > data.size() - at) return false;
        at += size;
        return true;
    }
};

struct MpuKey {
    std::uint32_t context = 0;
    std::uint16_t component = 0;
    std::uint32_t sequence = 0;
    bool operator<(const MpuKey& other) const {
        return std::tie(context, component, sequence) <
               std::tie(other.context, other.component, other.sequence);
    }
    bool operator==(const MpuKey&) const = default;
};

struct ItemKey {
    MpuKey mpu;
    std::uint32_t item = 0;
    bool operator<(const ItemKey& other) const {
        return std::tie(mpu.context, mpu.component, mpu.sequence, item) <
               std::tie(other.mpu.context, other.mpu.component, other.mpu.sequence, other.item);
    }
    bool operator==(const ItemKey&) const = default;
};

struct MpuMap {
    std::uint32_t transaction_id = 0;
    std::uint32_t download_id = 0;
    std::uint32_t index_item_id = 0;
    std::vector<std::uint16_t> node_tags;
    std::optional<std::uint16_t> mpu_node_tag;
    bool operator==(const MpuMap&) const = default;
};

struct FileRecord {
    std::uint32_t item_id = 0;
    std::uint32_t item_size = 0;
    std::uint8_t version = 0;
    std::string filename;
    std::string content_type;
    std::uint8_t compression_type = 0xff;
    std::optional<std::uint32_t> original_size;
};

struct FileTarget {
    std::string path;
    std::string content_type;
    std::uint32_t item_size = 0;
    std::uint8_t version = 0;
    std::uint8_t compression_type = 0xff;
    std::optional<std::uint32_t> original_size;
    bool operator==(const FileTarget&) const = default;
};

struct PublishedItem {
    std::uint32_t transaction_id = 0;
    std::uint32_t download_id = 0;
    std::uint8_t version = 0;
    std::string path;
};

template <typename Table>
struct SectionSet {
    std::optional<std::uint8_t> version;
    std::optional<std::uint8_t> last_section;
    std::vector<std::optional<Table>> sections;

    void add(const Table& table) {
        if (!version.has_value() || *version != table.version ||
            !last_section.has_value() || *last_section != table.last_section_number) {
            version = table.version;
            last_section = table.last_section_number;
            sections.clear();
            sections.resize(static_cast<std::size_t>(table.last_section_number) + 1U);
        }
        if (table.section_number > *last_section) return;
        sections[table.section_number] = table;
    }

    bool complete() const {
        return version.has_value() && last_section.has_value() && !sections.empty() &&
            std::all_of(sections.begin(), sections.end(), [](const auto& section) {
                return section.has_value();
            });
    }

    std::vector<Table> values() const {
        std::vector<Table> result;
        result.reserve(sections.size());
        for (const auto& section : sections) {
            if (section.has_value()) result.push_back(*section);
        }
        return result;
    }
};

struct ActiveCatalogue {
    std::uint16_t source_packet_id = 0;
    std::uint8_t session_id = 0;
    std::uint8_t directory_version = 0;
    std::uint8_t management_version = 0;
    std::vector<DataDirectoryTable> directories;
    std::vector<DataAssetManagementTable> management;
};

struct CandidateCatalogue {
    std::uint16_t source_packet_id = 0;
    std::uint8_t session_id = 0;
    SectionSet<DataDirectoryTable> directories;
    SectionSet<DataAssetManagementTable> management;
};

struct ContextCatalogueState {
    std::optional<ActiveCatalogue> active;
    std::map<std::pair<std::uint16_t, std::uint8_t>, CandidateCatalogue> candidates;
};

std::optional<std::string> safe_path(const std::string& value) {
    if (value.empty()) return std::string{};
    std::string normalized;
    std::size_t begin = 0;
    while (begin <= value.size()) {
        const auto end = value.find('/', begin);
        const auto part = value.substr(begin, end == std::string::npos
            ? std::string::npos : end - begin);
        if (part == "..") return std::nullopt;
        if (!part.empty() && part != ".") {
            if (!normalized.empty()) normalized.push_back('/');
            normalized += part;
        }
        if (end == std::string::npos) break;
        begin = end + 1;
    }
    return normalized;
}

std::optional<std::string> join_path(const std::string& first, const std::string& second,
                                     const std::string& third = {}) {
    const auto a = safe_path(first);
    const auto b = safe_path(second);
    const auto c = safe_path(third);
    if (!a.has_value() || !b.has_value() || !c.has_value()) return std::nullopt;
    std::string result;
    for (const auto* part : {&*a, &*b, &*c}) {
        if (part->empty()) continue;
        if (!result.empty()) result.push_back('/');
        result += *part;
    }
    return result;
}

bool parse_index(const std::vector<std::uint8_t>& data, std::vector<FileRecord>& records) {
    Cursor cursor{data};
    std::uint16_t count = 0;
    if (!cursor.u16(count)) return false;
    records.reserve(count);
    for (std::uint32_t index = 0; index < count; ++index) {
        FileRecord record;
        std::uint8_t filename_size = 0;
        std::uint8_t checksum = 0;
        std::uint8_t type_size = 0;
        if (!cursor.u32(record.item_id) || !cursor.u32(record.item_size) ||
            !cursor.u8(record.version) || !cursor.u8(filename_size) ||
            !cursor.text(filename_size, record.filename) || !cursor.u8(checksum)) {
            return false;
        }
        if ((checksum & 0x80U) != 0 && !cursor.skip(4)) return false;
        if (!cursor.u8(type_size) || !cursor.text(type_size, record.content_type) ||
            !cursor.u8(record.compression_type)) {
            return false;
        }
        if (record.compression_type != 0xff) {
            std::uint32_t original_size = 0;
            if (!cursor.u32(original_size)) return false;
            record.original_size = original_size;
        }
        records.push_back(std::move(record));
    }
    return cursor.at == data.size();
}

std::string application_key(const ApplicationInfo& info) {
    return std::to_string(info.context_id) + ':' + std::to_string(info.application_type) + ':' +
        std::to_string(info.organization_id) + ':' + std::to_string(info.application_id);
}

ApplicationLifecycleState application_lifecycle(const ApplicationInfo& info,
                                                const bool entry_ready) {
    switch (info.control_code) {
    case 0x01:
        return entry_ready ? ApplicationLifecycleState::AutostartReady
                           : ApplicationLifecycleState::AutostartPending;
    case 0x02: return ApplicationLifecycleState::Present;
    case 0x04: return ApplicationLifecycleState::Killed;
    case 0x05:
        return entry_ready ? ApplicationLifecycleState::Prefetched
                           : ApplicationLifecycleState::Prefetching;
    default: return ApplicationLifecycleState::Unsupported;
    }
}

} // namespace

class ApplicationResourceAssembler::Impl {
public:
    Impl(ApplicationResourceSink& sink, Limits limits)
        : sink_(sink), limits_(std::move(limits)) {}

    void application(const ApplicationInfo& info) {
        if (!limits_.collect_application_resources) return;
        auto& state = applications_[application_key(info)];
        state.application = info;
        state.state = ApplicationCollectionState::Collecting;
        state.resource_count = resource_count(info.context_id);
        state.entry_ready = entry_ready(info);
        if (state.entry_ready) state.state = ApplicationCollectionState::Ready;
        state.lifecycle = application_lifecycle(info, state.entry_ready);
        sink_.onApplicationState(state);
    }

    void directory(const DataDirectoryTable& table) {
        if (!limits_.collect_application_resources) return;
        if (!table.current_next) return;
        auto& state = catalogues_[table.context_id];
        if (state.active.has_value() &&
            state.active->source_packet_id == table.source_packet_id &&
            state.active->session_id == table.session_id &&
            state.active->directory_version == table.version) {
            return;
        }
        auto& candidate = state.candidates[{table.source_packet_id, table.session_id}];
        candidate.source_packet_id = table.source_packet_id;
        candidate.session_id = table.session_id;
        candidate.directories.add(table);
        try_commit_catalogue(table.context_id, table.source_packet_id,
                             table.session_id);
    }

    void management(const DataAssetManagementTable& table) {
        if (!limits_.collect_application_resources) return;
        if (!table.current_next) return;
        auto& state = catalogues_[table.context_id];
        if (state.active.has_value() &&
            state.active->source_packet_id == table.source_packet_id &&
            state.active->session_id == table.session_id &&
            state.active->management_version == table.version) {
            return;
        }
        auto& candidate = state.candidates[{table.source_packet_id, table.session_id}];
        candidate.source_packet_id = table.source_packet_id;
        candidate.session_id = table.session_id;
        candidate.management.add(table);
        try_commit_catalogue(table.context_id, table.source_packet_id,
                             table.session_id);
    }

    void unit(const DataUnit& value) {
        if (!limits_.collect_application_resources) return;
        const ItemKey key{{value.context_id, value.component_tag, value.mpu_sequence_number},
                          value.item_id};
        if (try_consume(key, value)) return;

        const auto existing = pending_.find(key);
        const auto previous_size = existing == pending_.end() ? 0U : existing->second.data.size();
        if (pending_.size() + (existing == pending_.end() ? 1U : 0U) >
                limits_.max_application_pending_units ||
            value.data.size() > limits_.max_application_pending_bytes -
                std::min(pending_bytes_, limits_.max_application_pending_bytes) + previous_size) {
            report(ErrorCode::ResourceLimit, value.input_offset, true,
                   "application resource pending-data limit exceeded");
            return;
        }
        pending_bytes_ -= previous_size;
        pending_[key] = value;
        pending_bytes_ += value.data.size();
        retry();
    }

    void reset() {
        catalogues_.clear();
        node_paths_.clear();
        mpu_maps_.clear();
        item_paths_.clear();
        pending_.clear();
        published_.clear();
        published_paths_.clear();
        superseded_mpus_.clear();
        deferred_item_removals_.clear();
        applications_.clear();
        pending_bytes_ = 0;
        sink_.onApplicationResourcesReset();
    }

    void drop_transient_keep_active() {
        for (auto& entry : catalogues_) entry.second.candidates.clear();
        pending_.clear();
        pending_bytes_ = 0;
    }

private:
    static MpuMap make_mpu_map(const DataAssetManagementTable& table,
                               const DataAssetMpu& mpu) {
        MpuMap map;
        map.transaction_id = table.transaction_id;
        map.download_id = table.download_id;
        map.index_item_id = mpu.index_item_id.value_or(0);
        for (const auto& item : mpu.items) map.node_tags.push_back(item.node_tag);
        for (std::size_t at = 0; at + 3 <= mpu.info.size();) {
            const auto length = static_cast<std::size_t>(mpu.info[at + 2]);
            if (length > mpu.info.size() - at - 3) break;
            if (length == 2) map.mpu_node_tag = be16(mpu.info.data() + at + 3);
            at += 3 + length;
        }
        return map;
    }

    bool build_catalogue(
        const std::uint32_t context,
        const std::vector<DataDirectoryTable>& directories,
        const std::vector<DataAssetManagementTable>& management,
        std::map<std::pair<std::uint32_t, std::uint16_t>, std::string>& nodes,
        std::map<MpuKey, MpuMap>& mpus) {
        std::set<std::string> base_paths;
        std::set<std::uint16_t> node_tags;
        for (const auto& table : directories) {
            const auto base = safe_path(table.base_path);
            if (!base.has_value() || !base_paths.insert(*base).second) {
                report(ErrorCode::MalformedInput, 0, true,
                       "application catalogue has an invalid or duplicate base directory");
                return false;
            }
            for (const auto& directory : table.directories) {
                const auto path = join_path(table.base_path, directory.path);
                if (!path.has_value() || !node_tags.insert(directory.node_tag).second) {
                    report(ErrorCode::MalformedInput, 0, true,
                           "application catalogue has an invalid path or duplicate node tag");
                    return false;
                }
                nodes[{context, directory.node_tag}] = *path;
            }
        }

        std::set<std::uint16_t> components;
        for (const auto& table : management) {
            if (!components.insert(table.component_tag).second) {
                report(ErrorCode::MalformedInput, 0, true,
                       "application catalogue repeats a component across DAMT sections");
                return false;
            }
            for (const auto& mpu : table.mpus) {
                const MpuKey key{context, table.component_tag, mpu.sequence_number};
                auto map = make_mpu_map(table, mpu);
                if (!mpus.emplace(key, map).second) {
                    report(ErrorCode::MalformedInput, 0, true,
                           "application catalogue repeats an MPU");
                    return false;
                }
                auto referenced_nodes = map.node_tags;
                if (referenced_nodes.empty() && map.mpu_node_tag.has_value()) {
                    referenced_nodes.push_back(*map.mpu_node_tag);
                }
                for (const auto node : referenced_nodes) {
                    if (nodes.find({context, node}) == nodes.end()) {
                        report(ErrorCode::MalformedInput, 0, true,
                               "DAMT MPU refers to a node absent from the complete DDMT");
                        return false;
                    }
                }
            }
        }
        return true;
    }

    void try_commit_catalogue(const std::uint32_t context,
                              const std::uint16_t source_packet_id,
                              const std::uint8_t session_id) {
        auto state_it = catalogues_.find(context);
        if (state_it == catalogues_.end()) return;
        auto& state = state_it->second;
        auto candidate_it = state.candidates.find({source_packet_id, session_id});
        if (candidate_it == state.candidates.end()) return;
        auto& candidate = candidate_it->second;
        const bool same_active_session = state.active.has_value() &&
            state.active->source_packet_id == source_packet_id &&
            state.active->session_id == session_id;
        if (!candidate.directories.complete() && !same_active_session) return;
        if (!candidate.management.complete() && !same_active_session) return;
        if (!candidate.directories.complete() && !candidate.management.complete()) return;

        const auto directories = candidate.directories.complete()
            ? candidate.directories.values() : state.active->directories;
        const auto management = candidate.management.complete()
            ? candidate.management.values() : state.active->management;
        const auto directory_version = candidate.directories.complete()
            ? *candidate.directories.version : state.active->directory_version;
        const auto management_version = candidate.management.complete()
            ? *candidate.management.version : state.active->management_version;

        std::map<std::pair<std::uint32_t, std::uint16_t>, std::string> new_nodes;
        std::map<MpuKey, MpuMap> new_mpus;
        if (!build_catalogue(context, directories, management, new_nodes, new_mpus)) return;
        commit_catalogue(context, new_nodes, new_mpus);
        state.active = ActiveCatalogue{source_packet_id, session_id, directory_version,
                                       management_version, directories, management};
        state.candidates.clear();
        retry();
    }

    void commit_catalogue(
        const std::uint32_t context,
        const std::map<std::pair<std::uint32_t, std::uint16_t>, std::string>& new_nodes,
        const std::map<MpuKey, MpuMap>& new_mpus) {
        std::set<std::uint16_t> changed_nodes;
        for (const auto& existing : node_paths_) {
            if (existing.first.first != context) continue;
            const auto replacement = new_nodes.find(existing.first);
            if (replacement == new_nodes.end() || replacement->second != existing.second) {
                changed_nodes.insert(existing.first.second);
            }
        }

        std::vector<MpuKey> retired;
        for (const auto& existing : mpu_maps_) {
            if (existing.first.context != context) continue;
            const auto replacement = new_mpus.find(existing.first);
            const bool changed_reference = std::any_of(
                existing.second.node_tags.begin(), existing.second.node_tags.end(),
                [&](const auto node) { return changed_nodes.contains(node); }) ||
                (existing.second.mpu_node_tag.has_value() &&
                 changed_nodes.contains(*existing.second.mpu_node_tag));
            if (replacement == new_mpus.end() || replacement->second != existing.second ||
                changed_reference) {
                retired.push_back(existing.first);
            }
        }
        for (const auto& key : retired) {
            if (new_mpus.find(key) != new_mpus.end()) {
                // Metadata for the same MPU key changed. Keep its last
                // published bytes until the replacement index/items arrive.
                continue;
            }
            const auto replacement = std::find_if(
                new_mpus.begin(), new_mpus.end(), [&](const auto& item) {
                    return item.first.context == key.context &&
                        item.first.component == key.component &&
                        (item.first.sequence >> 16U) == (key.sequence >> 16U);
                });
            if (replacement != new_mpus.end() && replacement->first != key) {
                superseded_mpus_[replacement->first].push_back(key);
            } else {
                erase_mpu_targets(key, false);
            }
        }

        for (auto it = node_paths_.begin(); it != node_paths_.end();) {
            if (it->first.first == context) it = node_paths_.erase(it);
            else ++it;
        }
        node_paths_.insert(new_nodes.begin(), new_nodes.end());
        for (auto it = mpu_maps_.begin(); it != mpu_maps_.end();) {
            if (it->first.context == context) it = mpu_maps_.erase(it);
            else ++it;
        }
        mpu_maps_.insert(new_mpus.begin(), new_mpus.end());
        update_application_states(context);
    }

    void retry() {
        bool progressed = true;
        while (progressed) {
            progressed = false;
            for (auto it = pending_.begin(); it != pending_.end();) {
                if (!try_consume(it->first, it->second)) {
                    ++it;
                    continue;
                }
                pending_bytes_ -= it->second.data.size();
                it = pending_.erase(it);
                progressed = true;
            }
        }
    }

    bool try_consume(const ItemKey& key, const DataUnit& unit) {
        const auto map_it = mpu_maps_.find(key.mpu);
        if (map_it == mpu_maps_.end()) return false;
        if (!unit.download_id.has_value() ||
            *unit.download_id != map_it->second.download_id) {
            report(ErrorCode::MalformedInput, unit.input_offset, true,
                   "application data-unit download_id does not match active DAMT");
            return true;
        }
        if (key.item == map_it->second.index_item_id) {
            return consume_index(key.mpu, unit, map_it->second);
        }
        const auto target = item_paths_.find(key);
        if (target == item_paths_.end()) return false;
        return publish(key, unit, map_it->second, target->second);
    }

    bool consume_index(const MpuKey& key, const DataUnit& unit, const MpuMap& map) {
        std::vector<FileRecord> records;
        if (!parse_index(unit.data, records)) {
            report(ErrorCode::MalformedInput, unit.input_offset, true,
                   "cannot parse application resource index item");
            return true;
        }
        auto node_tags = map.node_tags;
        if (node_tags.empty() && map.mpu_node_tag.has_value()) {
            node_tags.assign(records.size(), *map.mpu_node_tag);
        }
        if (records.size() != node_tags.size()) {
            report(ErrorCode::MalformedInput, unit.input_offset, true,
                   "application resource index mapping count mismatch");
            return true;
        }
        for (const auto node : node_tags) {
            if (node_paths_.find({key.context, node}) == node_paths_.end()) return false;
        }
        const auto previous_count = static_cast<std::size_t>(std::count_if(
            item_paths_.begin(), item_paths_.end(), [&](const auto& item) {
                return item.first.mpu == key;
            }));
        if (item_paths_.size() - previous_count + records.size() >
            limits_.max_application_resources) {
            report(ErrorCode::ResourceLimit, unit.input_offset, true,
                   "application resource catalogue limit exceeded");
            return true;
        }

        std::map<std::uint32_t, FileTarget> targets;
        for (std::size_t index = 0; index < records.size(); ++index) {
            const auto directory = node_paths_.at({key.context, node_tags[index]});
            const auto path = join_path(directory, records[index].filename);
            if (!path.has_value()) {
                report(ErrorCode::MalformedInput, unit.input_offset, true,
                       "broadcast file path escapes its virtual root");
                continue;
            }
            const auto inserted = targets.emplace(records[index].item_id, FileTarget{
                *path, records[index].content_type, records[index].item_size,
                records[index].version, records[index].compression_type,
                records[index].original_size});
            if (!inserted.second) {
                report(ErrorCode::MalformedInput, unit.input_offset, true,
                       "application resource index repeats an item id");
                return true;
            }
        }

        std::vector<ItemKey> retired;
        for (const auto& existing : item_paths_) {
            if (!(existing.first.mpu == key)) continue;
            const auto replacement = targets.find(existing.first.item);
            if (replacement == targets.end()) {
                retired.push_back(existing.first);
            }
        }
        if (!retired.empty()) {
            auto& deferred = deferred_item_removals_[key];
            deferred.insert(deferred.end(), retired.begin(), retired.end());
        }
        for (auto it = item_paths_.begin(); it != item_paths_.end();) {
            if (it->first.mpu == key) it = item_paths_.erase(it);
            else ++it;
        }
        for (const auto& target : targets) {
            item_paths_[{key, target.first}] = target.second;
        }
        finish_mpu_update_if_ready(key);
        update_application_states(key.context);
        return true;
    }

    bool publish(const ItemKey& key, const DataUnit& unit, const MpuMap& map,
                 const FileTarget& target) {
        const PublishedItem signature{map.transaction_id, map.download_id,
                                      target.version, target.path};
        const auto emitted = published_.find(key);
        if (emitted != published_.end() &&
            emitted->second.transaction_id == signature.transaction_id &&
            emitted->second.download_id == signature.download_id &&
            emitted->second.version == signature.version &&
            emitted->second.path == signature.path) {
            return true;
        }
        if (unit.data.size() != target.item_size) {
            report(ErrorCode::MalformedInput, unit.input_offset, true,
                   "application resource size does not match its index");
            return true;
        }

        std::vector<std::uint8_t> bytes;
        if (target.compression_type == 0xff) {
            if (unit.data.size() > limits_.max_application_resource) {
                report(ErrorCode::ResourceLimit, unit.input_offset, true,
                       "application resource exceeds size limit");
                return true;
            }
            bytes = unit.data;
        } else {
            if (target.compression_type != 0 || !target.original_size.has_value()) {
                report(ErrorCode::UnsupportedFeature, unit.input_offset, true,
                       "unsupported application resource compression type");
                return true;
            }
            if (*target.original_size > limits_.max_application_resource) {
                report(ErrorCode::ResourceLimit, unit.input_offset, true,
                       "decompressed application resource exceeds size limit");
                return true;
            }
            bytes.resize(*target.original_size);
            auto output_size = static_cast<uLongf>(bytes.size());
            const auto status = uncompress(bytes.data(), &output_size, unit.data.data(),
                                           static_cast<uLong>(unit.data.size()));
            if (status != Z_OK || output_size != bytes.size()) {
                report(ErrorCode::MalformedInput, unit.input_offset, true,
                       "cannot decompress application resource");
                return true;
            }
        }

        const auto previous = published_.find(key);
        if (previous != published_.end() && previous->second.path != target.path) {
            const auto old_path = previous->second.path;
            const auto path_entry = published_paths_.find({key.mpu.context, old_path});
            if (path_entry != published_paths_.end() && path_entry->second == key) {
                published_paths_.erase(path_entry);
                sink_.onApplicationResourceRemoved(ApplicationResourceRemoval{
                    key.mpu.context, key.mpu.component,
                    previous->second.transaction_id, previous->second.download_id,
                    key.mpu.sequence, key.item, old_path});
            }
        }
        published_[key] = signature;
        published_paths_[{key.mpu.context, target.path}] = key;
        ApplicationResource resource;
        resource.context_id = key.mpu.context;
        resource.component_tag = key.mpu.component;
        resource.transaction_id = map.transaction_id;
        resource.download_id = map.download_id;
        resource.mpu_sequence_number = key.mpu.sequence;
        resource.item_id = key.item;
        resource.version = target.version;
        resource.path = target.path;
        resource.content_type = target.content_type;
        resource.data = std::move(bytes);
        sink_.onApplicationResource(std::move(resource));
        finish_mpu_update_if_ready(key.mpu);
        update_application_states(key.mpu.context);
        return true;
    }

    void finish_mpu_update_if_ready(const MpuKey& key) {
        const auto superseded = superseded_mpus_.find(key);
        const auto deferred = deferred_item_removals_.find(key);
        if (superseded == superseded_mpus_.end() &&
            deferred == deferred_item_removals_.end()) return;
        bool has_target = false;
        for (const auto& item : item_paths_) {
            if (!(item.first.mpu == key)) continue;
            has_target = true;
            const auto published = published_.find(item.first);
            if (published == published_.end() ||
                published->second.path != item.second.path ||
                published->second.version != item.second.version) {
                return;
            }
        }
        (void)has_target;
        if (superseded != superseded_mpus_.end()) {
            for (const auto& old : superseded->second) erase_mpu_targets(old, false);
            superseded_mpus_.erase(superseded);
        }
        if (deferred != deferred_item_removals_.end()) {
            for (const auto& old : deferred->second) erase_item_target(old, false);
            deferred_item_removals_.erase(deferred);
        }
    }

    bool entry_ready(const ApplicationInfo& info) const {
        const auto entry = safe_path(info.entry_path);
        if (!entry.has_value()) return false;
        if (published_paths_.find({info.context_id, *entry}) != published_paths_.end()) {
            return true;
        }
        for (const auto& transport : info.transport_urls) {
            if (transport.find("://") != std::string::npos) continue;
            const auto candidate = join_path(transport, *entry);
            if (candidate.has_value() &&
                published_paths_.find({info.context_id, *candidate}) != published_paths_.end()) {
                return true;
            }
        }
        return false;
    }

    std::size_t resource_count(const std::uint32_t context) const {
        return static_cast<std::size_t>(std::count_if(
            published_paths_.begin(), published_paths_.end(), [context](const auto& item) {
                return item.first.first == context;
            }));
    }

    void update_application_states(const std::uint32_t context) {
        for (auto& item : applications_) {
            auto& state = item.second;
            if (state.application.context_id != context) continue;
            const auto ready = entry_ready(state.application);
            const auto count = resource_count(context);
            const auto lifecycle = application_lifecycle(state.application, ready);
            if (state.entry_ready == ready && state.resource_count == count &&
                state.lifecycle == lifecycle) {
                continue;
            }
            state.entry_ready = ready;
            state.resource_count = count;
            state.state = ready ? ApplicationCollectionState::Ready
                                : ApplicationCollectionState::Collecting;
            state.lifecycle = lifecycle;
            sink_.onApplicationState(state);
        }
    }

    void erase_item_target(const ItemKey& key, const bool update_states = true) {
        item_paths_.erase(key);
        const auto published = published_.find(key);
        if (published != published_.end()) {
            const auto path = published->second.path;
            const auto path_entry = published_paths_.find({key.mpu.context, path});
            if (path_entry != published_paths_.end() && path_entry->second == key) {
                published_paths_.erase(path_entry);
                sink_.onApplicationResourceRemoved(ApplicationResourceRemoval{
                    key.mpu.context, key.mpu.component, published->second.transaction_id,
                    published->second.download_id, key.mpu.sequence, key.item, path});
            }
            published_.erase(published);
        }
        const auto pending = pending_.find(key);
        if (pending != pending_.end()) {
            pending_bytes_ -= pending->second.data.size();
            pending_.erase(pending);
        }
        if (update_states) update_application_states(key.mpu.context);
    }

    void erase_mpu_targets(const MpuKey& key, const bool update_states = true) {
        for (auto it = item_paths_.begin(); it != item_paths_.end();) {
            if (!(it->first.mpu < key) && !(key < it->first.mpu)) it = item_paths_.erase(it);
            else ++it;
        }
        for (auto it = published_.begin(); it != published_.end();) {
            if (!(it->first.mpu < key) && !(key < it->first.mpu)) {
                const auto path = it->second.path;
                const auto path_entry = published_paths_.find({key.context, path});
                if (path_entry != published_paths_.end() &&
                    path_entry->second == it->first) {
                    published_paths_.erase(path_entry);
                    sink_.onApplicationResourceRemoved(ApplicationResourceRemoval{
                        key.context, key.component, it->second.transaction_id,
                        it->second.download_id, key.sequence, it->first.item, path});
                }
                it = published_.erase(it);
            } else {
                ++it;
            }
        }
        for (auto it = pending_.begin(); it != pending_.end();) {
            if (!(it->first.mpu < key) && !(key < it->first.mpu)) {
                pending_bytes_ -= it->second.data.size();
                it = pending_.erase(it);
            } else {
                ++it;
            }
        }
        if (update_states) update_application_states(key.context);
    }

    void report(const ErrorCode code, const std::uint64_t offset, const bool recoverable,
                std::string message) {
        sink_.onApplicationResourceError(Error{code, offset, recoverable, std::move(message)});
    }

    ApplicationResourceSink& sink_;
    Limits limits_;
    std::map<std::uint32_t, ContextCatalogueState> catalogues_;
    std::map<std::pair<std::uint32_t, std::uint16_t>, std::string> node_paths_;
    std::map<MpuKey, MpuMap> mpu_maps_;
    std::map<ItemKey, FileTarget> item_paths_;
    std::map<ItemKey, DataUnit> pending_;
    std::map<ItemKey, PublishedItem> published_;
    std::map<std::pair<std::uint32_t, std::string>, ItemKey> published_paths_;
    std::map<MpuKey, std::vector<MpuKey>> superseded_mpus_;
    std::map<MpuKey, std::vector<ItemKey>> deferred_item_removals_;
    std::unordered_map<std::string, ApplicationState> applications_;
    std::size_t pending_bytes_ = 0;
};

ApplicationResourceAssembler::ApplicationResourceAssembler(ApplicationResourceSink& sink,
                                                           Limits limits)
    : impl_(std::make_unique<Impl>(sink, std::move(limits))) {}

ApplicationResourceAssembler::~ApplicationResourceAssembler() = default;
ApplicationResourceAssembler::ApplicationResourceAssembler(ApplicationResourceAssembler&&) noexcept =
    default;
ApplicationResourceAssembler& ApplicationResourceAssembler::operator=(
    ApplicationResourceAssembler&&) noexcept = default;

void ApplicationResourceAssembler::onApplication(const ApplicationInfo& value) {
    impl_->application(value);
}
void ApplicationResourceAssembler::onDataDirectoryTable(const DataDirectoryTable& value) {
    impl_->directory(value);
}
void ApplicationResourceAssembler::onDataAssetManagementTable(
    const DataAssetManagementTable& value) {
    impl_->management(value);
}
void ApplicationResourceAssembler::onDataUnit(const DataUnit& value) {
    impl_->unit(value);
}
void ApplicationResourceAssembler::dropTransientKeepActive() {
    impl_->drop_transient_keep_active();
}
void ApplicationResourceAssembler::reset() { impl_->reset(); }

class ApplicationResourceStore::Impl {
public:
    struct StoredResource {
        std::shared_ptr<const ApplicationResource> resource;
        std::uint64_t generation = 0;
    };

    mutable std::mutex mutex;
    mutable std::condition_variable changed;
    std::map<std::pair<std::uint32_t, std::string>, StoredResource> resources;
    std::unordered_map<std::string, ApplicationState> application_states;
    std::uint64_t generation = 0;
};

ApplicationResourceStore::ApplicationResourceStore() : impl_(std::make_unique<Impl>()) {}
ApplicationResourceStore::~ApplicationResourceStore() = default;
ApplicationResourceStore::ApplicationResourceStore(ApplicationResourceStore&&) noexcept = default;
ApplicationResourceStore& ApplicationResourceStore::operator=(
    ApplicationResourceStore&&) noexcept = default;

void ApplicationResourceStore::onApplicationState(const ApplicationState& state) {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->application_states[application_key(state.application)] = state;
    ++impl_->generation;
    impl_->changed.notify_all();
}

void ApplicationResourceStore::onApplicationResource(ApplicationResource&& resource) {
    auto shared = std::make_shared<const ApplicationResource>(std::move(resource));
    const auto key = std::make_pair(shared->context_id, shared->path);
    std::lock_guard<std::mutex> lock(impl_->mutex);
    ++impl_->generation;
    impl_->resources[key] =
        Impl::StoredResource{std::move(shared), impl_->generation};
    impl_->changed.notify_all();
}

void ApplicationResourceStore::onApplicationResourceRemoved(
    const ApplicationResourceRemoval& removal) {
    const auto normalized = safe_path(removal.path);
    if (!normalized.has_value()) return;
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->resources.erase({removal.context_id, *normalized});
    ++impl_->generation;
    impl_->changed.notify_all();
}

void ApplicationResourceStore::onApplicationResourcesReset() { clear(); }

std::shared_ptr<const ApplicationResource> ApplicationResourceStore::get(
    const std::uint32_t context_id, const std::string& path) const {
    const auto normalized = safe_path(path);
    if (!normalized.has_value()) return {};
    std::lock_guard<std::mutex> lock(impl_->mutex);
    const auto found = impl_->resources.find({context_id, *normalized});
    return found == impl_->resources.end() ? nullptr : found->second.resource;
}

std::shared_ptr<const ApplicationResource> ApplicationResourceStore::waitFor(
    const std::uint32_t context_id, const std::string& path,
    const std::chrono::milliseconds timeout) const {
    const auto normalized = safe_path(path);
    if (!normalized.has_value()) return {};
    std::unique_lock<std::mutex> lock(impl_->mutex);
    const auto key = std::make_pair(context_id, *normalized);
    if (!impl_->changed.wait_for(lock, timeout, [&] {
            return impl_->resources.find(key) != impl_->resources.end();
        })) {
        return {};
    }
    return impl_->resources.at(key).resource;
}

std::vector<ApplicationResourceMetadata> ApplicationResourceStore::list(
    const std::optional<std::uint32_t> context_id) const {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    std::vector<ApplicationResourceMetadata> result;
    result.reserve(impl_->resources.size());
    for (const auto& item : impl_->resources) {
        const auto& resource = *item.second.resource;
        if (context_id.has_value() && resource.context_id != *context_id) continue;
        result.push_back(ApplicationResourceMetadata{
            resource.context_id, resource.component_tag, resource.transaction_id,
            resource.download_id, resource.mpu_sequence_number, resource.item_id,
            resource.version, resource.path, resource.content_type, resource.data.size(),
            item.second.generation});
    }
    return result;
}

std::vector<ApplicationState> ApplicationResourceStore::applications() const {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    std::vector<ApplicationState> result;
    result.reserve(impl_->application_states.size());
    for (const auto& item : impl_->application_states) result.push_back(item.second);
    return result;
}

std::optional<std::string> ApplicationResourceStore::entryPath(
    const std::uint32_t context_id) const {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    for (const auto& item : impl_->application_states) {
        const auto& state = item.second;
        if (state.application.context_id != context_id || !state.entry_ready) continue;
        const auto entry = safe_path(state.application.entry_path);
        if (!entry.has_value()) continue;
        if (impl_->resources.find({context_id, *entry}) != impl_->resources.end()) {
            return entry;
        }
        for (const auto& transport : state.application.transport_urls) {
            if (transport.find("://") != std::string::npos) continue;
            const auto candidate = join_path(transport, *entry);
            if (candidate.has_value() &&
                impl_->resources.find({context_id, *candidate}) != impl_->resources.end()) {
                return candidate;
            }
        }
    }
    return std::nullopt;
}

std::uint64_t ApplicationResourceStore::generation() const {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    return impl_->generation;
}

void ApplicationResourceStore::clear() {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->resources.clear();
    impl_->application_states.clear();
    ++impl_->generation;
    impl_->changed.notify_all();
}

} // namespace aribtlv
