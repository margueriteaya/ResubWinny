#pragma once

#include <cstddef>
#include <cstdint>
#include <chrono>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include <aribtlv/types.hpp>

namespace aribtlv {

enum class ApplicationCollectionState {
    Discovered,
    Collecting,
    Ready,
};

enum class ApplicationLifecycleState {
    Unsupported,
    AutostartPending,
    AutostartReady,
    Present,
    Prefetching,
    Prefetched,
    Killed,
};

struct ApplicationState {
    ApplicationInfo application;
    ApplicationCollectionState state = ApplicationCollectionState::Discovered;
    ApplicationLifecycleState lifecycle = ApplicationLifecycleState::Unsupported;
    std::size_t resource_count = 0;
    bool entry_ready = false;
};

struct ApplicationResource {
    std::uint32_t context_id = 0;
    std::uint16_t component_tag = 0;
    std::uint32_t transaction_id = 0;
    std::uint32_t download_id = 0;
    std::uint32_t mpu_sequence_number = 0;
    std::uint32_t item_id = 0;
    std::uint8_t version = 0;
    std::string path;
    std::string content_type;
    std::vector<std::uint8_t> data;
};

struct ApplicationResourceRemoval {
    std::uint32_t context_id = 0;
    std::uint16_t component_tag = 0;
    std::uint32_t transaction_id = 0;
    std::uint32_t download_id = 0;
    std::uint32_t mpu_sequence_number = 0;
    std::uint32_t item_id = 0;
    std::string path;
};

struct ApplicationResourceMetadata {
    std::uint32_t context_id = 0;
    std::uint16_t component_tag = 0;
    std::uint32_t transaction_id = 0;
    std::uint32_t download_id = 0;
    std::uint32_t mpu_sequence_number = 0;
    std::uint32_t item_id = 0;
    std::uint8_t version = 0;
    std::string path;
    std::string content_type;
    std::size_t size = 0;
    std::uint64_t generation = 0;
};

class ApplicationResourceSink {
public:
    virtual ~ApplicationResourceSink() = default;
    virtual void onApplicationState(const ApplicationState&) {}
    virtual void onApplicationResource(ApplicationResource&&) {}
    virtual void onApplicationResourceRemoved(const ApplicationResourceRemoval&) {}
    virtual void onApplicationResourcesReset() {}
    virtual void onApplicationResourceError(const Error&) {}
};

// Reassembles the directory, asset-management and data-unit signalling into
// browser-ready files. It is synchronous and does not create worker threads;
// callers may run the owning Demuxer on their own worker/thread.
class ApplicationResourceAssembler {
public:
    ApplicationResourceAssembler(ApplicationResourceSink&, Limits = {});
    ~ApplicationResourceAssembler();

    ApplicationResourceAssembler(ApplicationResourceAssembler&&) noexcept;
    ApplicationResourceAssembler& operator=(ApplicationResourceAssembler&&) noexcept;
    ApplicationResourceAssembler(const ApplicationResourceAssembler&) = delete;
    ApplicationResourceAssembler& operator=(const ApplicationResourceAssembler&) = delete;

    void onApplication(const ApplicationInfo&);
    void onDataDirectoryTable(const DataDirectoryTable&);
    void onDataAssetManagementTable(const DataAssetManagementTable&);
    void onDataUnit(const DataUnit&);
    // Discard section/data-unit acquisition that cannot safely cross an input
    // discontinuity while retaining the last complete catalogue and all files
    // already published from it. This is the application-resource counterpart
    // of an in-source reposition/seek.
    void dropTransientKeepActive();
    void reset();

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

// Thread-safe in-memory virtual resource catalogue suitable for a loopback
// HTTP adapter. WASM callers will normally keep the same events in a JS Map.
class ApplicationResourceStore final : public ApplicationResourceSink {
public:
    ApplicationResourceStore();
    ~ApplicationResourceStore() override;

    ApplicationResourceStore(ApplicationResourceStore&&) noexcept;
    ApplicationResourceStore& operator=(ApplicationResourceStore&&) noexcept;
    ApplicationResourceStore(const ApplicationResourceStore&) = delete;
    ApplicationResourceStore& operator=(const ApplicationResourceStore&) = delete;

    void onApplicationState(const ApplicationState&) override;
    void onApplicationResource(ApplicationResource&&) override;
    void onApplicationResourceRemoved(const ApplicationResourceRemoval&) override;
    void onApplicationResourcesReset() override;

    std::shared_ptr<const ApplicationResource> get(std::uint32_t context_id,
                                                   const std::string& path) const;
    std::shared_ptr<const ApplicationResource> waitFor(
        std::uint32_t context_id, const std::string& path,
        std::chrono::milliseconds timeout) const;
    std::vector<ApplicationResourceMetadata> list(
        std::optional<std::uint32_t> context_id = std::nullopt) const;
    std::vector<ApplicationState> applications() const;
    std::optional<std::string> entryPath(std::uint32_t context_id) const;
    std::uint64_t generation() const;
    void clear();

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace aribtlv
