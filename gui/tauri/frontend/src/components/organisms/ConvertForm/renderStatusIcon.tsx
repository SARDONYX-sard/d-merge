import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import ErrorIcon from '@mui/icons-material/Error';
import CircularProgress from '@mui/material/CircularProgress';

export const renderStatusIcon = (status: number) => {
  switch (status) {
    case 1:
      return <CircularProgress size={20} />;
    case 2:
      return <CheckCircleIcon color='success' />;
    case 3:
      return <ErrorIcon color='error' />;
    default:
      return undefined;
  }
};
