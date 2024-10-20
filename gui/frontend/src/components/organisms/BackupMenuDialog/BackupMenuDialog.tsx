import CloseIcon from '@mui/icons-material/Close';
import { Button, type ButtonProps, Dialog } from '@mui/material';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import IconButton from '@mui/material/IconButton';
import List from '@mui/material/List';

import { useTranslation } from '@/components/hooks/useTranslation';
import { OBJECT } from '@/lib/object-utils';
import type { Cache, CacheKey } from '@/lib/storage';

import { CacheItem } from './CacheItem';
import { CheckBoxControls } from './CheckBoxControls';
import { useCheckBoxState } from './useCheckBoxState';

import type { Dispatch, ReactNode, SetStateAction } from 'react';

export type DialogClickHandler = (checkedKeys: readonly CacheKey[]) => void;
export type BackupMenuDialogProps = {
  buttonName: string;
  cacheItems: Cache;
  inDialogClick: DialogClickHandler;
  open: boolean;
  setOpen: Dispatch<SetStateAction<boolean>>;
  title: ReactNode;
};

export const BackupMenuDialog = ({
  buttonName,
  cacheItems,
  inDialogClick,
  open,
  setOpen,
  title,
}: BackupMenuDialogProps) => {
  const handleClose = () => setOpen(false);
  const { isAllChecked, isPubAllChecked, checked, handleToggleItem, handleCheckAll, handleCheckPubAll } =
    useCheckBoxState(cacheItems);

  return (
    <Dialog fullWidth={true} maxWidth='md' onClose={handleClose} open={open}>
      <DialogTitle>
        {title}
        <IconButton
          aria-label='close'
          onClick={handleClose}
          sx={{ position: 'absolute', right: 8, top: 8, color: (theme) => theme.palette.grey[500] }}
        >
          <CloseIcon />
        </IconButton>

        <CheckBoxControls
          isAllChecked={isAllChecked}
          isPubAllChecked={isPubAllChecked}
          onAllCheck={handleCheckAll}
          onPubCheck={handleCheckPubAll}
        />
      </DialogTitle>

      <DialogContent dividers={true}>
        <List sx={{ minWidth: 360, bgcolor: 'background.paper' }}>
          {OBJECT.entries(cacheItems).map(([key, value]) => (
            <CacheItem
              key={key}
              onToggle={handleToggleItem(key)}
              selected={checked.includes(key)}
              title={key}
              value={value}
            />
          ))}
        </List>
      </DialogContent>

      <DialogActions>
        <Button onClick={() => inDialogClick(checked)}>{buttonName}</Button>
        <CancelButton onClick={handleClose} />
      </DialogActions>
    </Dialog>
  );
};

const CancelButton = ({ ...props }: ButtonProps) => {
  const { t } = useTranslation();
  return <Button {...props}>{t('cancel-btn')}</Button>;
};
