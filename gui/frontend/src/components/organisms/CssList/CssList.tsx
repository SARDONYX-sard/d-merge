import { type SelectChangeEvent, Tooltip } from '@mui/material';

import { useTranslation } from '@/components/hooks/useTranslation';
import { SelectWithLabel } from '@/components/molecules/SelectWithLabel';
import { useCssContext } from '@/components/providers/CssProvider';
import { CSS } from '@/lib/css';

export const CssList = () => {
  const { t } = useTranslation();
  const { preset, setPreset, setCss } = useCssContext();

  const handleChange = (e: SelectChangeEvent<string>) => {
    const presetN = CSS.normalize(e.target.value);
    setPreset(presetN);
    setCss(CSS.css.get(presetN));
  };

  const title = (
    <>
      <p>{t('css_preset.list_tooltip')}</p>
      <p>{t('css_preset.list_tooltip_edit_warning')}</p>
    </>
  );

  return (
    <Tooltip placement='right-end' title={title}>
      <SelectWithLabel
        label={t('css_preset.list_label')}
        menuItems={[
          { value: '0', label: t('css_preset.items.custom') },
          { value: '1', label: t('css_preset.items.preset1') },
          { value: '2', label: t('css_preset.items.preset2') },
          { value: '3', label: t('css_preset.items.preset3') },
          { value: '4', label: t('css_preset.items.preset4') },
        ]}
        onChange={handleChange}
        value={preset}
      />
    </Tooltip>
  );
};
