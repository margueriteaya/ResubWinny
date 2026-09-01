#include <aribtlv/application_resources.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <string>
#include <utility>
#include <vector>

#include <zlib.h>

namespace {

struct TestSink final : aribtlv::ApplicationResourceSink {
    std::vector<aribtlv::ApplicationState> states;
    std::vector<aribtlv::ApplicationResource> resources;
    std::vector<aribtlv::ApplicationResourceRemoval> removals;
    std::vector<aribtlv::Error> errors;
    std::size_t resets = 0;

    void onApplicationState(const aribtlv::ApplicationState& value) override {
        states.push_back(value);
    }
    void onApplicationResource(aribtlv::ApplicationResource&& value) override {
        resources.push_back(std::move(value));
    }
    void onApplicationResourceRemoved(
        const aribtlv::ApplicationResourceRemoval& value) override {
        removals.push_back(value);
    }
    void onApplicationResourcesReset() override { ++resets; }
    void onApplicationResourceError(const aribtlv::Error& value) override {
        errors.push_back(value);
    }
};

[[noreturn]] void fail(const std::string& message) {
    std::cerr << "FAIL: " << message << '\n';
    std::exit(1);
}

void check(const bool condition, const std::string& message) {
    if (!condition) fail(message);
}

void append_u16(std::vector<std::uint8_t>& value, const std::uint32_t number) {
    value.push_back(static_cast<std::uint8_t>(number >> 8U));
    value.push_back(static_cast<std::uint8_t>(number));
}

void append_u32(std::vector<std::uint8_t>& value, const std::uint32_t number) {
    value.push_back(static_cast<std::uint8_t>(number >> 24U));
    value.push_back(static_cast<std::uint8_t>(number >> 16U));
    value.push_back(static_cast<std::uint8_t>(number >> 8U));
    value.push_back(static_cast<std::uint8_t>(number));
}

std::vector<std::uint8_t> index_item(const std::uint32_t item_id,
                                     const std::uint32_t stored_size,
                                     const std::uint8_t version,
                                     const std::string& filename,
                                     const std::string& content_type,
                                     const std::uint8_t compression_type = 0xff,
                                     const std::uint32_t original_size = 0) {
    std::vector<std::uint8_t> value;
    append_u16(value, 1);
    append_u32(value, item_id);
    append_u32(value, stored_size);
    value.push_back(version);
    value.push_back(static_cast<std::uint8_t>(filename.size()));
    value.insert(value.end(), filename.begin(), filename.end());
    value.push_back(0);
    value.push_back(static_cast<std::uint8_t>(content_type.size()));
    value.insert(value.end(), content_type.begin(), content_type.end());
    value.push_back(compression_type);
    if (compression_type != 0xff) append_u32(value, original_size);
    return value;
}

std::vector<std::uint8_t> two_index_items(const std::uint32_t first_item,
                                          const std::string& first_name,
                                          const std::uint32_t second_item,
                                          const std::string& second_name,
                                          const std::uint32_t size,
                                          const std::uint8_t version) {
    auto first = index_item(first_item, size, version, first_name, "text/plain");
    const auto second = index_item(second_item, size, version, second_name, "text/plain");
    first[1] = 2;
    first.insert(first.end(), second.begin() + 2, second.end());
    return first;
}

aribtlv::DataDirectoryTable directory(const std::uint32_t context,
                                       const std::uint8_t session = 0,
                                       const std::uint8_t version = 0,
                                       const std::uint8_t section = 0,
                                       const std::uint8_t last_section = 0,
                                       const std::uint16_t node_tag = 7,
                                       const std::string& base_path = "/sh4/60/001",
                                       const std::string& node_path = "top/source") {
    aribtlv::DataDirectoryTable value;
    value.context_id = context;
    value.session_id = session;
    value.version = version;
    value.section_number = section;
    value.last_section_number = last_section;
    value.base_path = base_path;
    aribtlv::DataDirectoryNode node;
    node.node_tag = node_tag;
    node.path = node_path;
    value.directories.push_back(std::move(node));
    return value;
}

aribtlv::DataAssetManagementTable management(const std::uint32_t context,
                                               const std::uint32_t transaction,
                                               const std::uint32_t download,
                                               const std::uint32_t sequence = 9,
                                               const std::uint8_t session = 0,
                                               const std::uint8_t version = 0,
                                               const std::uint8_t section = 0,
                                               const std::uint8_t last_section = 0,
                                               const std::uint16_t component = 0x40,
                                               const std::uint16_t node_tag = 7,
                                               const bool include_mpu = true) {
    aribtlv::DataAssetManagementTable value;
    value.context_id = context;
    value.session_id = session;
    value.version = version;
    value.section_number = section;
    value.last_section_number = last_section;
    value.transaction_id = transaction;
    value.component_tag = component;
    value.download_id = download;
    if (!include_mpu) return value;
    aribtlv::DataAssetMpu mpu;
    mpu.sequence_number = sequence;
    mpu.index_item = true;
    mpu.index_item_id = 99;
    aribtlv::DataAssetItem item;
    item.node_tag = node_tag;
    item.item_id = 5;
    mpu.items.push_back(std::move(item));
    value.mpus.push_back(std::move(mpu));
    return value;
}

aribtlv::DataUnit unit(const std::uint32_t context, const std::uint32_t item,
                        std::vector<std::uint8_t> bytes,
                        const std::uint32_t sequence = 9,
                        const std::uint32_t download = 20,
                        const std::uint16_t component = 0x40) {
    aribtlv::DataUnit value;
    value.context_id = context;
    value.component_tag = component;
    value.mpu_sequence_number = sequence;
    value.item_id = item;
    value.download_id = download;
    value.data = std::move(bytes);
    return value;
}

aribtlv::ApplicationInfo application(const std::uint32_t context) {
    aribtlv::ApplicationInfo value;
    value.context_id = context;
    value.application_type = 1;
    value.organization_id = 2;
    value.application_id = 3;
    value.version = 1;
    value.control_code = 0x01;
    value.entry_path = "top/source/index.html";
    value.transport_urls.push_back("sh4/60/001/");
    return value;
}

void test_out_of_order_collection_and_update() {
    TestSink sink;
    aribtlv::ApplicationResourceAssembler assembler(sink);
    const std::vector<std::uint8_t> first{'o', 'n', 'e'};

    assembler.onApplication(application(1));
    assembler.onDataUnit(unit(1, 5, first));
    assembler.onDataUnit(unit(1, 99, index_item(5, first.size(), 1,
        "index.html", "text/html")));
    assembler.onDataAssetManagementTable(management(1, 10, 20));
    check(sink.resources.empty(), "resource was published before its directory arrived");
    assembler.onDataDirectoryTable(directory(1));

    check(sink.resources.size() == 1, "out-of-order resource was not published");
    check(sink.resources[0].path == "sh4/60/001/top/source/index.html",
          "resource path was not normalized");
    check(sink.resources[0].data == first, "resource bytes changed during assembly");
    check(!sink.states.empty() && sink.states.back().entry_ready &&
              sink.states.back().state == aribtlv::ApplicationCollectionState::Ready &&
              sink.states.back().lifecycle ==
                  aribtlv::ApplicationLifecycleState::AutostartReady,
          "entry resource did not make the application ready");

    auto prefetched = application(1);
    prefetched.control_code = 0x05;
    assembler.onApplication(prefetched);
    check(sink.states.back().lifecycle ==
              aribtlv::ApplicationLifecycleState::Prefetched,
          "PREFETCH application did not retain its ready resource state");
    auto killed = application(1);
    killed.control_code = 0x04;
    assembler.onApplication(killed);
    check(sink.states.back().lifecycle == aribtlv::ApplicationLifecycleState::Killed,
          "KILL application control was not exposed separately from collection state");

    assembler.onDataUnit(unit(1, 5, first));
    check(sink.resources.size() == 1, "carousel duplicate was emitted twice");

    const std::vector<std::uint8_t> second{'t', 'w', 'o'};
    assembler.onDataAssetManagementTable(management(1, 11, 21, 9, 0, 1));
    check(sink.removals.empty() && sink.states.back().entry_ready,
          "DAMT update removed the old readable resource before replacement bytes arrived");
    assembler.onDataUnit(unit(1, 99, index_item(5, second.size(), 2,
        "index.html", "text/html"), 9, 21));
    assembler.onDataUnit(unit(1, 5, second, 9, 21));
    check(sink.resources.size() == 2 && sink.resources.back().version == 2 &&
              sink.resources.back().data == second,
          "new application resource version was not published");

    assembler.onDataAssetManagementTable(management(1, 11, 21, 10, 0, 2));
    check(sink.removals.empty(),
          "new MPU version retired the preceding bytes before its files were complete");
    assembler.onDataUnit(unit(1, 99, index_item(5, second.size(), 3,
        "index.html", "text/html"), 10, 21));
    assembler.onDataUnit(unit(1, 5, second, 10, 21));
    check(sink.resources.size() == 3 && sink.removals.empty(),
          "complete replacement MPU did not atomically supersede the same VFS path");

    assembler.reset();
    check(sink.resets == 1, "reset was not propagated to the resource store");
}

void test_virtual_resource_store() {
    aribtlv::ApplicationResourceStore store;
    auto state = aribtlv::ApplicationState{};
    state.application = application(4);
    state.entry_ready = true;
    state.state = aribtlv::ApplicationCollectionState::Ready;
    store.onApplicationState(state);
    aribtlv::ApplicationResource resource;
    resource.context_id = 4;
    resource.path = "sh4/60/001/top/source/index.html";
    resource.content_type = "text/html";
    resource.version = 3;
    resource.data = {'o', 'k'};
    store.onApplicationResource(std::move(resource));

    const auto found = store.get(4, "/sh4/60/001/top/source/index.html");
    check(found && found->data == std::vector<std::uint8_t>({'o', 'k'}),
          "virtual resource lookup failed");
    const auto files = store.list(4);
    check(files.size() == 1 && files[0].size == 2 && files[0].version == 3,
          "virtual resource listing omitted metadata");
    check(store.waitFor(4, files[0].path, std::chrono::milliseconds(1)) != nullptr,
          "waitFor did not return an available resource");
    check(store.entryPath(4) == files[0].path,
          "virtual resource store did not resolve the ready application entry");
    check(store.get(4, "../escape") == nullptr,
          "virtual resource store accepted path traversal");
    store.onApplicationResourceRemoved(aribtlv::ApplicationResourceRemoval{
        4, 0, 0, 0, 0, 0, "sh4/60/001/top/source/index.html"});
    check(store.list(4).empty(), "virtual resource removal left a stale VFS entry");
    store.clear();
    check(store.list().empty(), "virtual resource store did not clear its session");
}

void test_compression_and_limit() {
    const std::vector<std::uint8_t> source{'c', 'o', 'm', 'p', 'r', 'e', 's', 's'};
    std::vector<std::uint8_t> compressed(compressBound(source.size()));
    auto compressed_size = static_cast<uLongf>(compressed.size());
    check(compress(compressed.data(), &compressed_size, source.data(), source.size()) == Z_OK,
          "test fixture compression failed");
    compressed.resize(compressed_size);

    TestSink sink;
    aribtlv::ApplicationResourceAssembler assembler(sink);
    assembler.onDataDirectoryTable(directory(2));
    assembler.onDataAssetManagementTable(management(2, 1, 1));
    assembler.onDataUnit(unit(2, 99, index_item(5, compressed.size(), 1,
        "index.html", "text/html", 0, source.size()), 9, 1));
    assembler.onDataUnit(unit(2, 5, compressed, 9, 1));
    check(sink.resources.size() == 1 && sink.resources[0].data == source,
          "zlib application resource was not decompressed");

    aribtlv::Limits limits;
    limits.max_application_resource = 3;
    TestSink limited_sink;
    aribtlv::ApplicationResourceAssembler limited(limited_sink, limits);
    limited.onDataDirectoryTable(directory(3));
    limited.onDataAssetManagementTable(management(3, 1, 1));
    limited.onDataUnit(unit(3, 99, index_item(5, source.size(), 1,
        "index.html", "text/html"), 9, 1));
    limited.onDataUnit(unit(3, 5, source, 9, 1));
    check(limited_sink.resources.empty() && !limited_sink.errors.empty() &&
              limited_sink.errors.back().code == aribtlv::ErrorCode::ResourceLimit,
          "resource size limit was not enforced");
}

void test_session_snapshot_and_reposition_lifecycle() {
    TestSink sink;
    aribtlv::ApplicationResourceAssembler assembler(sink);
    const std::vector<std::uint8_t> old_bytes{'o', 'l', 'd'};

    assembler.onDataDirectoryTable(directory(10, 1, 1));
    assembler.onDataAssetManagementTable(management(10, 30, 30, 9, 1, 1));
    assembler.onDataUnit(unit(10, 99, index_item(5, old_bytes.size(), 1,
        "old.html", "text/html"), 9, 30));
    assembler.onDataUnit(unit(10, 5, old_bytes, 9, 30));
    check(sink.resources.size() == 1, "initial active catalogue did not publish");

    assembler.onDataDirectoryTable(directory(
        10, 2, 2, 0, 1, 7, "/next", "first"));
    assembler.onDataAssetManagementTable(management(10, 31, 31, 10, 2, 2));
    check(sink.removals.empty(),
          "partial new-session DDMT replaced the active catalogue");

    assembler.dropTransientKeepActive();
    assembler.onDataDirectoryTable(directory(
        10, 2, 2, 1, 1, 8, "/next-extra", "second"));
    check(sink.removals.empty(),
          "post-reposition orphan section committed against discarded state");

    assembler.onDataDirectoryTable(directory(
        10, 2, 2, 0, 1, 7, "/next", "first"));
    assembler.onDataAssetManagementTable(management(10, 31, 31, 10, 2, 2));
    check(sink.removals.empty(),
          "complete new catalogue retired old resources before its MPU was ready");
    assembler.onDataUnit(unit(10, 99, index_item(5, old_bytes.size(), 2,
        "new.html", "text/html"), 10, 31));
    assembler.onDataUnit(unit(10, 5, old_bytes, 10, 31));
    check(sink.removals.size() == 1 && sink.removals.back().path.ends_with("old.html"),
          "ready replacement MPU did not retire the superseded path");
}

void test_same_session_single_table_update() {
    TestSink sink;
    aribtlv::ApplicationResourceAssembler assembler(sink);
    const std::vector<std::uint8_t> bytes{'o', 'k'};

    assembler.onDataDirectoryTable(directory(11, 4, 3));
    assembler.onDataAssetManagementTable(management(11, 40, 40, 9, 4, 3));
    assembler.onDataUnit(unit(11, 99, index_item(5, bytes.size(), 1,
        "index.html", "text/html"), 9, 40));
    assembler.onDataUnit(unit(11, 5, bytes, 9, 40));
    check(sink.resources.size() == 1, "same-session fixture did not publish");

    assembler.onDataAssetManagementTable(management(11, 41, 41, 10, 4, 4));
    check(sink.removals.empty(),
          "same-session DAMT update did not retain the old MPU while collecting");
    assembler.onDataUnit(unit(11, 99, index_item(5, bytes.size(), 2,
        "next.html", "text/html"), 10, 41));
    assembler.onDataUnit(unit(11, 5, bytes, 10, 41));
    check(sink.removals.size() == 1,
          "complete same-session replacement did not reuse active DDMT and retire old path");
}

void test_download_id_and_snapshot_diff() {
    TestSink sink;
    aribtlv::ApplicationResourceAssembler assembler(sink);
    const std::vector<std::uint8_t> bytes{'x'};

    assembler.onDataDirectoryTable(directory(12, 5, 1));
    auto flexible_management = management(12, 50, 50, 9, 5, 1);
    flexible_management.mpus[0].items.clear();
    flexible_management.mpus[0].info = {0, 0, 2, 0, 7};
    assembler.onDataAssetManagementTable(flexible_management);
    assembler.onDataUnit(unit(12, 99, index_item(5, bytes.size(), 1,
        "wrong.html", "text/html"), 9, 49));
    check(sink.resources.empty() && !sink.errors.empty(),
          "mismatched data-unit download_id was accepted");
    assembler.onDataUnit(unit(12, 99, index_item(5, bytes.size(), 1,
        "right.html", "text/html"), 9, 50));
    assembler.onDataUnit(unit(12, 5, bytes, 9, 50));
    check(sink.resources.size() == 1 && sink.resources.back().path.ends_with("right.html"),
          "matching data-unit download_id was not accepted");

    assembler.onDataUnit(unit(12, 99,
        two_index_items(5, "right.html", 6, "removed.txt", bytes.size(), 2),
        9, 50));
    assembler.onDataUnit(unit(12, 5, bytes, 9, 50));
    assembler.onDataUnit(unit(12, 6, bytes, 9, 50));
    check(sink.resources.size() == 3, "two-item MPU fixture did not publish both files");

    assembler.onDataUnit(unit(12, 99, index_item(5, bytes.size(), 3,
        "right.html", "text/html"), 9, 50));
    check(std::none_of(sink.removals.begin(), sink.removals.end(), [](const auto& removal) {
              return removal.item_id == 6 && removal.path.ends_with("removed.txt");
          }), "new index deleted an old item before replacement MPU completion");
    assembler.onDataUnit(unit(12, 5, bytes, 9, 50));
    check(std::any_of(sink.removals.begin(), sink.removals.end(), [](const auto& removal) {
              return removal.item_id == 6 && removal.path.ends_with("removed.txt");
          }), "complete new MPU did not delete an item absent from its index");

    const auto removals_before_catalogue = sink.removals.size();
    assembler.onDataDirectoryTable(directory(
        12, 5, 2, 0, 0, 8, "/replacement", "app"));
    assembler.onDataAssetManagementTable(
        management(12, 51, 51, 10, 5, 3, 0, 0, 0x41, 8));
    check(sink.removals.size() > removals_before_catalogue,
          "catalogue diff left resources from a removed node/component/MPU");
}

} // namespace

int main() {
    test_out_of_order_collection_and_update();
    test_virtual_resource_store();
    test_compression_and_limit();
    test_session_snapshot_and_reposition_lifecycle();
    test_same_session_single_table_update();
    test_download_id_and_snapshot_diff();
    std::cout << "application resource tests passed\n";
    return 0;
}
