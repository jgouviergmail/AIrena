import { useTranslation } from "react-i18next";
import type { OceanScores } from "@/lib/persona-types";

interface Props {
  ocean: OceanScores;
  onChange: (ocean: OceanScores) => void;
}

const TRAITS = ["O", "C", "E", "A", "N"] as const;

export function OceanSliders({ ocean, onChange }: Props) {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 gap-2">
      {TRAITS.map((trait) => (
        <div key={trait} className="space-y-1">
          <div className="flex items-center justify-between">
            <label className="text-xs text-muted-foreground">
              {t(`personaEditor.ocean_${trait}`)}
            </label>
            <span className="text-xs font-mono text-foreground">
              {ocean[trait]}
            </span>
          </div>
          <input
            type="range"
            min={1}
            max={10}
            step={1}
            value={ocean[trait]}
            onChange={(e) =>
              onChange({ ...ocean, [trait]: parseInt(e.target.value) })
            }
            className="w-full accent-primary"
          />
        </div>
      ))}
    </div>
  );
}
