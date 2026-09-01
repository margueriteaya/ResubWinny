#include <aribtlv/aribtlv.h>

int main(void)
{
    aribtlv_callbacks callbacks;
    aribtlv_config config;
    aribtlv_hlg_sdr_lut_info lut_info;
    aribtlv_callbacks_init(&callbacks);
    aribtlv_config_init(&config);
    aribtlv_demuxer *demuxer = aribtlv_demuxer_create(&config, &callbacks, 0);
    if (!demuxer || aribtlv_version() != ARIBTLV_VERSION_INT ||
        aribtlv_hlg_sdr_lut_describe(
            ARIBTLV_HLG_SDR_LUT_DISPLAY, &lut_info) != ARIBTLV_OK ||
        lut_info.dimension != 33)
        return 1;
    aribtlv_demuxer_destroy(demuxer);
    return 0;
}
