#include <aribtlv/hlg_sdr_tone_mapping.hpp>

#include <algorithm>
#include <cstdlib>
#include <cmath>
#include <iostream>

namespace {

void check(const bool condition, const char* message) {
    if (condition) return;
    std::cerr << "FAIL: " << message << '\n';
    std::exit(1);
}

double lut_channel(const aribtlv::HlgSdrColorLut& lut,
                   const std::size_t red, const std::size_t green,
                   const std::size_t blue, const std::size_t channel) {
    const auto columns = lut.width / lut.size;
    const auto x = (blue % columns) * lut.size + red;
    const auto y = (blue / columns) * lut.size + green;
    const auto offset = (y * lut.width + x) * 4U + channel;
    return static_cast<double>(lut.rgba[offset]) / 255.0;
}

aribtlv::HlgSdrRgb sample_lut_trilinear(
    const aribtlv::HlgSdrColorLut& lut, const aribtlv::HlgSdrRgb input) {
    const auto coordinate = [size = lut.size](const double value) {
        return value * static_cast<double>(size - 1U);
    };
    const double red = coordinate(input.red);
    const double green = coordinate(input.green);
    const double blue = coordinate(input.blue);
    const auto red0 = static_cast<std::size_t>(std::floor(red));
    const auto green0 = static_cast<std::size_t>(std::floor(green));
    const auto blue0 = static_cast<std::size_t>(std::floor(blue));
    const auto red1 = std::min(red0 + 1U, lut.size - 1U);
    const auto green1 = std::min(green0 + 1U, lut.size - 1U);
    const auto blue1 = std::min(blue0 + 1U, lut.size - 1U);
    const auto lerp = [](const double lower, const double upper,
                         const double amount) {
        return lower + (upper - lower) * amount;
    };
    const auto channel = [&](const std::size_t index) {
        const auto slice = [&](const std::size_t blue_index) {
            const double lower = lerp(
                lut_channel(lut, red0, green0, blue_index, index),
                lut_channel(lut, red1, green0, blue_index, index),
                red - static_cast<double>(red0));
            const double upper = lerp(
                lut_channel(lut, red0, green1, blue_index, index),
                lut_channel(lut, red1, green1, blue_index, index),
                red - static_cast<double>(red0));
            return lerp(lower, upper, green - static_cast<double>(green0));
        };
        return lerp(slice(blue0), slice(blue1),
                    blue - static_cast<double>(blue0));
    };
    return {channel(0), channel(1), channel(2)};
}

} // namespace

int main() {
    using aribtlv::detail::map_hlg_sdr_signal;
    using aribtlv::detail::map_hlg_sdr_display_signal;

    check(map_hlg_sdr_signal(0.0) == 0.0, "black anchor changed");
    check(map_hlg_sdr_signal(0.4) == 0.4, "40% anchor changed");
    check(map_hlg_sdr_signal(0.75) == 0.84, "75% anchor changed");
    check(map_hlg_sdr_signal(0.79) == 0.94, "79% shoulder changed");
    check(map_hlg_sdr_signal(0.90) == 0.985, "90% shoulder changed");
    check(map_hlg_sdr_signal(1.0) == 1.0, "100% shoulder changed");
    check(map_hlg_sdr_display_signal(0.40) == 0.40,
          "measured SDR correction unexpectedly lifts midtones");

    const auto lut = aribtlv::hlg_sdr_tone_mapping_lut();
    check(lut.front() == 0 && lut.back() == 255, "LUT endpoints changed");
    for (std::size_t index = 1; index < lut.size(); ++index) {
        check(lut[index] >= lut[index - 1], "LUT is not monotonic");
    }

    const auto black = aribtlv::detail::map_hlg_sdr_display_rgb({0.0, 0.0, 0.0});
    check(black.red == 0.0 && black.green == 0.0 && black.blue == 0.0,
          "RGB mapper changed black");
    const auto neutral = aribtlv::detail::map_hlg_sdr_display_rgb({0.5, 0.5, 0.5});
    check(neutral.red == neutral.green && neutral.green == neutral.blue,
          "RGB mapper tinted neutral grey");
    const auto colour = aribtlv::detail::map_hlg_sdr_display_rgb({0.8, 0.4, 0.2});
    check(colour.red / colour.green == 2.0 && colour.green / colour.blue == 2.0,
          "RGB mapper changed unclipped RGB ratios");

    const auto color_lut = aribtlv::hlg_sdr_color_lut();
    check(color_lut.size == aribtlv::kHlgSdrColorLutSize,
          "3D LUT size changed");
    check(color_lut.width % color_lut.size == 0U &&
              color_lut.height % color_lut.size == 0U &&
              color_lut.width / color_lut.size *
                  (color_lut.height / color_lut.size) >= color_lut.size,
          "3D LUT texture layout is invalid");
    check(color_lut.rgba.size() == color_lut.width * color_lut.height * 4U,
          "3D LUT byte count is invalid");
    check(color_lut.rgba[0] == 0U && color_lut.rgba[1] == 0U &&
              color_lut.rgba[2] == 0U && color_lut.rgba[3] == 255U,
          "3D LUT black entry changed");
    check(lut_channel(color_lut, color_lut.size - 1U,
                      color_lut.size - 1U, color_lut.size - 1U, 0U) == 1.0 &&
              lut_channel(color_lut, color_lut.size - 1U,
                          color_lut.size - 1U, color_lut.size - 1U, 1U) == 1.0 &&
              lut_channel(color_lut, color_lut.size - 1U,
                          color_lut.size - 1U, color_lut.size - 1U, 2U) == 1.0 &&
              lut_channel(color_lut, color_lut.size - 1U,
                          color_lut.size - 1U, color_lut.size - 1U, 3U) == 1.0,
          "3D LUT white entry changed");
    double maximum_error = 0.0;
    for (unsigned red = 0; red <= 20; ++red) {
        for (unsigned green = 0; green <= 20; ++green) {
            for (unsigned blue = 0; blue <= 20; ++blue) {
                const aribtlv::HlgSdrRgb input{
                    static_cast<double>(red) / 20.0,
                    static_cast<double>(green) / 20.0,
                    static_cast<double>(blue) / 20.0,
                };
                const auto expected = aribtlv::detail::map_hlg_sdr_display_rgb(input);
                const auto actual = sample_lut_trilinear(color_lut, input);
                maximum_error = std::max({maximum_error,
                    std::abs(expected.red - actual.red),
                    std::abs(expected.green - actual.green),
                    std::abs(expected.blue - actual.blue)});
            }
        }
    }
    check(maximum_error <= 3.0 / 255.0,
          "3D LUT interpolation error exceeds three 8-bit levels");

    check(aribtlv::detail::prototype_sdr_luma_fit(0.0) == 0.0 &&
              aribtlv::detail::prototype_sdr_luma_fit(1.0) == 1.0,
          "prototype luma fit changed black or white");
    check(std::abs(aribtlv::detail::prototype_sdr_luma_fit(0.5480) -
                   0.4163) < 0.001 &&
              std::abs(aribtlv::detail::prototype_sdr_luma_fit(0.9208) -
                       0.8724) < 0.001,
          "prototype luma fit missed its simulcast anchors");
    double previous_fitted = 0.0;
    double maximum_fit_step = 0.0;
    for (unsigned index = 0; index <= 1000; ++index) {
        const double fitted = aribtlv::detail::prototype_sdr_luma_fit(
            static_cast<double>(index) / 1000.0);
        check(fitted + 1e-12 >= previous_fitted,
              "prototype luma fit is not monotonic");
        maximum_fit_step = std::max(maximum_fit_step, fitted - previous_fitted);
        previous_fitted = fitted;
    }
    check(maximum_fit_step < 0.004,
          "prototype luma fit contains a narrow highlight shoulder");

    const auto luma709 = [](const aribtlv::HlgSdrRgb color) {
        return 0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue;
    };
    const aribtlv::HlgSdrRgb ordinary{0.50, 0.45, 0.40};
    const auto mapped_ordinary = aribtlv::detail::soft_map_bt709_gamut(ordinary);
    check(mapped_ordinary.red == ordinary.red &&
              mapped_ordinary.green == ordinary.green &&
              mapped_ordinary.blue == ordinary.blue,
          "soft gamut map changed an interior BT.709 colour");

    const aribtlv::HlgSdrRgb gold{1.20, 0.85, 0.10};
    const auto mapped_gold = aribtlv::detail::soft_map_bt709_gamut(gold);
    check(mapped_gold.red > 0.0 && mapped_gold.red < 1.0 &&
              mapped_gold.green > 0.0 && mapped_gold.green < 1.0 &&
              mapped_gold.blue > 0.0 && mapped_gold.blue < 1.0,
          "soft gamut map hard-clipped a bright gold colour");
    check(std::abs(luma709(mapped_gold) - luma709(gold)) < 1e-12,
          "soft gamut map changed linear luminance");
    const double gold_luma = luma709(gold);
    const double gold_red_scale =
        (mapped_gold.red - gold_luma) / (gold.red - gold_luma);
    check(std::abs(mapped_gold.green - gold_luma -
                   gold_red_scale * (gold.green - gold_luma)) < 1e-12 &&
              std::abs(mapped_gold.blue - gold_luma -
                       gold_red_scale * (gold.blue - gold_luma)) < 1e-12,
          "soft gamut map changed the linear RGB hue direction");

    constexpr double sample_luma = 0.60;
    static constexpr aribtlv::HlgSdrRgb gold_direction{
        0.40, -0.05, -0.6825484764542936,
    };
    const auto along_gold = [](const double amount) {
        return aribtlv::HlgSdrRgb{
            sample_luma + amount * gold_direction.red,
            sample_luma + amount * gold_direction.green,
            sample_luma + amount * gold_direction.blue,
        };
    };
    const auto mapped_near_gold =
        aribtlv::detail::soft_map_bt709_gamut(along_gold(0.90));
    const auto mapped_far_gold =
        aribtlv::detail::soft_map_bt709_gamut(along_gold(1.30));
    check(mapped_far_gold.red > mapped_near_gold.red &&
              mapped_far_gold.red < 1.0 && mapped_far_gold.blue > 0.0,
          "soft gamut map collapsed distinct bright gold colours");
    const auto prototype_black =
        aribtlv::detail::map_hlg_sdr_prototype_rgb({0.0, 0.0, 0.0});
    check(prototype_black.red == 0.0 && prototype_black.green == 0.0 &&
              prototype_black.blue == 0.0,
          "prototype mapper changed black");
    const auto prototype_mid =
        aribtlv::detail::map_hlg_sdr_prototype_rgb({0.5, 0.5, 0.5});
    check(prototype_mid.red > 0.58 && prototype_mid.red < 0.60 &&
              std::abs(prototype_mid.red - prototype_mid.green) < 0.0001 &&
              std::abs(prototype_mid.green - prototype_mid.blue) < 0.0001,
          "prototype mapper does not apply the calibrated mid-grey anchor");
    const auto prototype_reference =
        aribtlv::detail::map_hlg_sdr_prototype_rgb({0.75, 0.75, 0.75});
    check(prototype_reference.red > 0.92 && prototype_reference.red < 0.94 &&
              std::abs(prototype_reference.red - prototype_reference.green) < 0.0001 &&
              std::abs(prototype_reference.green - prototype_reference.blue) < 0.0001,
          "prototype mapper does not apply the calibrated reference anchor");
    const auto prototype_white =
        aribtlv::detail::map_hlg_sdr_prototype_rgb({1.0, 1.0, 1.0});
    check(prototype_white.red > 0.995 && prototype_white.green > 0.995 &&
              prototype_white.blue > 0.995,
          "prototype mapper does not fit peak white into the browser canvas");

    const auto prototype_lut = aribtlv::hlg_sdr_prototype_color_lut();
    check(prototype_lut.size == aribtlv::kHlgSdrPrototypeColorLutSize &&
              prototype_lut.rgba.size() ==
                  prototype_lut.width * prototype_lut.height * 4U,
          "prototype 3D LUT layout is invalid");
    const auto prototype_mid_lut = sample_lut_trilinear(
        prototype_lut, {0.5, 0.5, 0.5});
    check(std::abs(prototype_mid_lut.red - prototype_mid.red) <= 4.0 / 255.0 &&
              std::abs(prototype_mid_lut.green - prototype_mid.green) <= 4.0 / 255.0 &&
              std::abs(prototype_mid_lut.blue - prototype_mid.blue) <= 4.0 / 255.0,
          "prototype LUT interpolation changed its mid-grey output");
    double prototype_maximum_error = 0.0;
    aribtlv::HlgSdrRgb prototype_maximum_error_input{};
    aribtlv::HlgSdrRgb prototype_maximum_error_expected{};
    aribtlv::HlgSdrRgb prototype_maximum_error_actual{};
    for (unsigned red = 0; red <= 20; ++red) {
        for (unsigned green = 0; green <= 20; ++green) {
            for (unsigned blue = 0; blue <= 20; ++blue) {
                const aribtlv::HlgSdrRgb input{
                    static_cast<double>(red) / 20.0,
                    static_cast<double>(green) / 20.0,
                    static_cast<double>(blue) / 20.0,
                };
                const auto expected =
                    aribtlv::detail::map_hlg_sdr_prototype_rgb(input);
                const auto actual = sample_lut_trilinear(prototype_lut, input);
                const double error = std::max({
                    std::abs(expected.red - actual.red),
                    std::abs(expected.green - actual.green),
                    std::abs(expected.blue - actual.blue)});
                if (error > prototype_maximum_error) {
                    prototype_maximum_error = error;
                    prototype_maximum_error_input = input;
                    prototype_maximum_error_expected = expected;
                    prototype_maximum_error_actual = actual;
                }
            }
        }
    }
    std::cout << "prototype maximum LUT error: "
              << prototype_maximum_error * 255.0 << " 8-bit levels at {"
              << prototype_maximum_error_input.red << ", "
              << prototype_maximum_error_input.green << ", "
              << prototype_maximum_error_input.blue << "}, expected {"
              << prototype_maximum_error_expected.red << ", "
              << prototype_maximum_error_expected.green << ", "
              << prototype_maximum_error_expected.blue << "}, actual {"
              << prototype_maximum_error_actual.red << ", "
              << prototype_maximum_error_actual.green << ", "
              << prototype_maximum_error_actual.blue << "}\n";
    check(prototype_maximum_error <= 8.0 / 255.0,
          "prototype 3D LUT interpolation error exceeds eight 8-bit levels");
    std::cout << "HLG-SDR C++ tone mapping tests passed\n";
}
