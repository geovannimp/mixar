import { createFormHook } from "@tanstack/react-form";
import { NumberField } from "@/components/settings/fields/NumberField";
import { SelectField } from "@/components/settings/fields/SelectField";
import { SliderField } from "@/components/settings/fields/SliderField";
import { ToggleField } from "@/components/settings/fields/ToggleField";
import { fieldContext, formContext } from "@/components/settings/form-context";
import { SettingsSaveButton } from "@/components/settings/SettingsSaveButton";

export const { useAppForm, withForm } = createFormHook({
  fieldContext,
  formContext,
  fieldComponents: {
    NumberField,
    SelectField,
    SliderField,
    ToggleField,
  },
  formComponents: {
    SaveButton: SettingsSaveButton,
  },
});
