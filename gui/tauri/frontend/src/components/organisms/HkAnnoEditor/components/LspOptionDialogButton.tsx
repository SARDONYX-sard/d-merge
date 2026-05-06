import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  FormControlLabel,
  Checkbox,
  Button,
  Box,
  Typography,
  Divider,
  Tooltip,
  IconButton,
} from '@mui/material';
import { useState } from 'react';
import { useEditorContext } from '../context/editorContext';
import { useTranslation } from '@/components/hooks/useTranslation';
import { OBJECT } from '@/lib/object-utils';

export const LspOptionDialogButton = () => {
  const [open, setOpen] = useState(false);
  const { t } = useTranslation();

  return (
    <>
      <Tooltip title={t('hkanno.lsp_options.title')}>
        <IconButton onClick={() => setOpen(true)}>⚙</IconButton>
      </Tooltip>

      <LspOptionDialog open={open} onClose={() => setOpen(false)} />
    </>
  );
};

const LspOptionDialog = ({ open, onClose }: { open: boolean; onClose: () => void }) => {
  const [state, dispatch] = useEditorContext();
  const { t } = useTranslation();
  const options = state.lspOptions;

  const handleChange = (key: keyof typeof options) => (e: React.ChangeEvent<HTMLInputElement>) => {
    dispatch({
      type: 'SET_LSP_OPTIONS',
      lspOptions: { [key]: e.target.checked },
    });
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth='xs' fullWidth>
      <DialogTitle>{t('hkanno.lsp_options.title')}</DialogTitle>

      <DialogContent dividers>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          {OBJECT.entries(options).map(([key, value]) => {
            const meta = OPTION_META_KEYS[key];

            return (
              <Box key={key}>
                <FormControlLabel
                  control={<Checkbox checked={value} onChange={handleChange(key)} />}
                  label={
                    <Box>
                      <Typography variant='body2'>{t(meta.label)}</Typography>
                      <Typography variant='caption' color='text.secondary'>
                        {t(meta.description)}
                      </Typography>
                    </Box>
                  }
                />
                <Divider sx={{ mt: 1 }} />
              </Box>
            );
          })}
        </Box>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>{t('hkanno.lsp_options.close_button_name')}</Button>
      </DialogActions>
    </Dialog>
  );
};

const OPTION_META_KEYS = {
  completion: {
    label: 'hkanno.lsp_options.completion.label',
    description: 'hkanno.lsp_options.completion.description',
  },
  codeAction: {
    label: 'hkanno.lsp_options.code_action.label',
    description: 'hkanno.lsp_options.code_action.description',
  },
  diagnostics: {
    label: 'hkanno.lsp_options.diagnostics.label',
    description: 'hkanno.lsp_options.diagnostics.description',
  },
  formatter: {
    label: 'hkanno.lsp_options.formatter.label',
    description: 'hkanno.lsp_options.formatter.description',
  },
  semanticTokens: {
    label: 'hkanno.lsp_options.semantic_tokens.label',
    description: 'hkanno.lsp_options.semantic_tokens.description',
  },
  hover: {
    label: 'hkanno.lsp_options.hover.label',
    description: 'hkanno.lsp_options.hover.description',
  },
  inlayHints: {
    label: 'hkanno.lsp_options.inlay_hints.label',
    description: 'hkanno.lsp_options.inlay_hints.description',
  },
  signatureHelp: {
    label: 'hkanno.lsp_options.signature_help.label',
    description: 'hkanno.lsp_options.signature_help.description',
  },
} as const;
