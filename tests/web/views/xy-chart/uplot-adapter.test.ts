import { describe, expect, it } from "vitest";
import { uplotOptions } from "../../../../src-web/views/xy-chart/uplot-adapter";

describe("uPlot adapter", () => {
  it("keeps non-time scale and series colors in one config", () => {
    const options = uplotOptions(720, 200, [{ label: "温度", stroke: "#34d399" }]);
    expect(options.scales?.x).toEqual({ time: false });
    expect(options.series).toEqual([{ label: "序号" }, { label: "温度", stroke: "#34d399" }]);
    expect(options.cursor?.drag).toEqual({ x: true, y: false });
  });
});
