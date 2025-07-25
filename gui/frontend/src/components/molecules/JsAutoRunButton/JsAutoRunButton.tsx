import type { FormControlLabelProps } from '@mui/material';
import { Checkbox, FormControlLabel, Tooltip } from '@mui/material';
import { useTranslation } from '@/components/hooks/useTranslation';
import { useJsContext } from '@/components/providers/JsProvider';

type Props = Omit<FormControlLabelProps, 'control' | 'label'>;
const CACHE_KEY = 'run-script';

export const JsAutoRunButton = ({ ...props }: Props) => {
  const { t } = useTranslation();
  const { runScript, setRunScript } = useJsContext();

  const title = (
    <>
      {t('custom_js.auto_run_tooltip')}
      <br />
      {t('custom_js.auto_run_tooltip_note')}
    </>
  );

  const handleClick = () => {
    if (runScript) {
      window.location.reload();
    }
    setRunScript(!runScript);
  };

  const label = t('custom_js.auto_run_label');

  return (
    <Tooltip title={title}>
      <FormControlLabel
        control={<Checkbox checked={runScript} name={`input-${CACHE_KEY}`} onClick={handleClick} />}
        label={label}
        {...props}
      />
    </Tooltip>
  );
};
