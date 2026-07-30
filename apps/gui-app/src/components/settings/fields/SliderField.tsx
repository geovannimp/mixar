import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Slider, SliderValue } from "@/components/ui/slider";
import { useFieldContext } from "@/components/settings/form-context";

interface SliderFieldProps {
  label: string;
  "aria-label"?: string;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  description?: string;
  formatValue?: (value: number) => string;
  /** When set, replaces the default SliderValue readout. */
  valueLabel?: string;
  transform?: (raw: number) => number;
}

export function SliderField({
  label,
  "aria-label": ariaLabel,
  min,
  max,
  step,
  disabled,
  description,
  formatValue,
  valueLabel,
  transform,
}: SliderFieldProps) {
  const field = useFieldContext<number>();

  return (
    <Field>
      <Slider
        aria-label={ariaLabel ?? label}
        disabled={disabled}
        value={field.state.value}
        min={min}
        max={max}
        step={step}
        onValueChange={(value) => {
          const next = Array.isArray(value) ? value[0] : value;
          if (next == null) {
            return;
          }
          field.handleChange(transform ? transform(next) : next);
        }}
      >
        <div className="mb-2 flex items-center justify-between gap-1">
          <FieldLabel className="font-medium text-sm">{label}</FieldLabel>
          {valueLabel != null ? (
            <span className="text-sm">{valueLabel}</span>
          ) : formatValue ? (
            <span className="text-sm">{formatValue(field.state.value)}</span>
          ) : (
            <SliderValue />
          )}
        </div>
      </Slider>
      {description ? <FieldDescription>{description}</FieldDescription> : null}
    </Field>
  );
}
