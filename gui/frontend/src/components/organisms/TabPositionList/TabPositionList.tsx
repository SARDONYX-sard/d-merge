import type { SelectChangeEvent } from '@mui/material/Select';
import { useTranslation } from '@/components/hooks/useTranslation';
import { SelectWithLabel } from '@/components/molecules/SelectWithLabel';
import { tabPosSchema, useTabContext } from '@/components/providers/TabProvider';

export const TabPositionList = () => {
  const { t } = useTranslation();
  const { tabPos, setTabPos } = useTabContext();

  const handleChange = ({ target }: SelectChangeEvent) => {
    setTabPos(tabPosSchema.parse(target.value));
  };

  const menuItems = [
    { value: 'top', label: t('tabs.positions.top') },
    { value: 'bottom', label: t('tabs.positions.bottom') },
  ] as const;

  return (
    <SelectWithLabel label={t('tabs.position_label')} menuItems={menuItems} onChange={handleChange} value={tabPos} />
  );
};
