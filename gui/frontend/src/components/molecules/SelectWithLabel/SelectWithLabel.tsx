import { FormControl, InputLabel, MenuItem, Select, type SelectChangeEvent } from '@mui/material';
import { type ComponentPropsWithRef, type JSX, type Ref, forwardRef, useId } from 'react';

import type { SxProps, Theme } from '@mui/system';

type MenuItems<V extends string> = {
  label: string;
  value: V;
};

type SelectWithLabelProps<V extends string, T extends MenuItems<V>> = {
  label: string;
  menuItems: readonly T[];
  onChange: (e: SelectChangeEvent<T['value']>) => void;
  sx?: SxProps<Theme>;
  value: T['value'];
  variant?: ComponentPropsWithRef<typeof FormControl>['variant'];
};

// NOTE: 1. `forwardRef` to get the ref and let it inherit the props so it will finally work when we want to use a `ToolTip`.
// TODO: However, `forwardRef` will be deprecated in React19 as it can be passed as component props.
export const SelectWithLabel = forwardRef(function SelectWithLabel<V extends string, T extends MenuItems<V>>(
  {
    label,
    menuItems,
    onChange,
    sx = { m: 1, minWidth: 110 },
    value,
    variant = 'filled',
    ...props // HACK: 2. We need to let props inherit in order for it to work when wrapped in `ToolTip`.
  }: SelectWithLabelProps<V, T>,
  ref: Ref<HTMLDivElement>,
) {
  const id = useId();
  const labelId = `${id}-label`;
  const selectId = `${id}-select`;

  return (
    <FormControl ref={ref} sx={sx} variant={variant} {...props}>
      <InputLabel id={labelId}>{label}</InputLabel>
      <Select MenuProps={{ disableScrollLock: true }} id={selectId} labelId={labelId} onChange={onChange} value={value}>
        {menuItems.map((menuItem) => (
          <MenuItem key={menuItem.value} value={menuItem.value}>
            {menuItem.label}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
  );
}) as SelectLevelFc; // HACK: Hacks for code completion with generics. The pattern of passing customRef was treated as an error, so we had to cast it.

/** For code completion with generics. The pattern of passing customRef was treated as an error, so we had to cast it. */
type SelectLevelFc = <V extends string, T extends MenuItems<V>>(
  props: SelectWithLabelProps<V, T> & { ref?: Ref<HTMLDivElement> },
) => JSX.Element;
