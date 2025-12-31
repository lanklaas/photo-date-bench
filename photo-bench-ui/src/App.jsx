import React, { useState, useEffect } from 'react';
import {
  Button,
  TextField,
  Container,
  Typography,
  Box,
  CircularProgress,
  LinearProgress,
  Tooltip,
  IconButton
} from '@mui/material';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { listen } from '@tauri-apps/api/event';
import { invoke } from "@tauri-apps/api/core";
import { open } from '@tauri-apps/plugin-dialog';
import CssBaseline from '@mui/material/CssBaseline';
import FolderIcon from '@mui/icons-material/Folder';
import GoogleIcon from '@mui/icons-material/Google';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
  },
});


function App() {
  const [sourceFolder, setSourceFolder] = useState('');
  const [targetFolder, setTargetFolder] = useState('');
  const [progress, setProgress] = useState(0);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isDone, setIsDone] = useState(false);
  const [files, setFiles] = useState([]);
  const [fileCount, setFileCount] = useState(0);


  // Listen for progress and task updates from Tauri
  useEffect(() => {
    if (isProcessing) {
      // Listener for progress updates
      const unlistenProgress = listen('process-progress', (event) => {
        const progress = event.payload;
        setProgress(progress);
      });

      const unlistenFile = listen('process-file', (event) => {
        const file = event.payload;
        setFiles([...files,file]);
      });

      const unlistenFileDone = listen('process-file-done', (event) => {
        const file = event.payload;
        setFileCount(c => c + 1);
        setFiles(files.filter(x=>x!=file))
      });

      // Listener for process completion
      const unlistenComplete = listen('process-complete', count => {
        setIsProcessing(false);
        setProgress(100);
        setIsDone(true)
      });

      return () => {
        unlistenProgress.then((f) => f());
        unlistenComplete.then((f) => f());
        unlistenFile.then((f) => f());
        unlistenFileDone.then((f) => f());
      };
    }
  }, [isProcessing]);


  const handleSelectFolder = async (folderSetFn) => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected && Array.isArray(selected)) {
        folderSetFn(selected[0]);
      } else if (typeof selected === 'string') {
        folderSetFn(selected);
      }
    } catch (error) {
      console.error('Failed to select folder:', error);
    }
  };

  const openFolder = async () => {
    await invoke('open_download_folder', { targetFolder });
  }

  const handleStartProcess = async () => {
    if (!(sourceFolder && targetFolder)) {
      alert('Please fill in all fields.');
      return;
    }

    setIsDone(false);
    setIsProcessing(true);
    setProgress(0);
    setFileCount(0);

    try {
      await invoke('process_images', { sourceFolder, targetFolder }); // Replace with your Tauri command
    } catch (error) {
      console.error('Process failed:', error);
      setIsProcessing(false);
    }
  };

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Container maxWidth="sm" style={{ marginTop: '5vh' }}>
        <Typography variant="h4" gutterBottom>
          Download PDFs van gmail
        </Typography>
        <Box component="form" noValidate autoComplete="on">
          <Box display="flex" alignItems="center" marginY={2}>
            <Tooltip title="Folder where the images to be processed are." placement="top-start">
              <TextField
                label="Source Folder"
                variant="outlined"
                fullWidth
                value={sourceFolder}
                InputProps={{
                  readOnly: true,
                }}
              />
            </Tooltip>
            <IconButton
                variant="contained"
                color="primary"
                onClick={() => handleSelectFolder(setSourceFolder)}
                // style={{ marginLeft: '10px', height: '56px' }}
                sx={{ml: 1}}
              >
                <FolderIcon/>
            </IconButton>
          </Box>
          <Box display="flex" alignItems="center" marginY={2}>
            <Tooltip title="Folder where the processed images should go to." placement="top-start">
              <TextField
                label="Target Folder"
                variant="outlined"
                fullWidth
                value={targetFolder}
                InputProps={{
                  readOnly: true,
                }}
              />
            </Tooltip>
            <IconButton
                variant="contained"
                color="primary"
                onClick={() => handleSelectFolder(setTargetFolder)}
                // style={{ marginLeft: '10px', height: '56px' }}
                sx={{ml: 1}}
              >
                <FolderIcon/>
            </IconButton>
          </Box>
          
          <Button
            variant="contained"
            color="primary"
            onClick={handleStartProcess}
            disabled={isProcessing}
          >
            {isProcessing ? (
              <CircularProgress size={24} color="inherit" />
            ) : (
              isDone ? 'Run Again' : 'Run'
            )}
          </Button>          
          {isProcessing && (
            <Box marginTop={4}>
              <LinearProgress variant="determinate" value={progress} />
              <Box
                display="flex"
                justifyContent="space-between"
                marginTop={1}
              >
                <Typography variant="caption">{progress}%</Typography>
                <Typography variant="caption">100%</Typography>
              </Box>
              <Typography variant="body1" gutterBottom>
                { files.length == 0 ? 'Scanning mails...' : 'Processing file: ' + files.join(',')}
              </Typography>
            </Box>
          )}
          {
            isDone && (
              <Box>
                <Typography variant="body1" gutterBottom marginTop={1}>
                  Done. {fileCount} files downloaded
                </Typography>
                <Button
                  variant="contained"
                  color="primary"
                  onClick={openFolder}
                  disabled={false}
                >
                  Open Folder
                </Button>
              </Box>
              )
          }
        </Box>
        
          
        
      </Container>
    </ThemeProvider>
  );
}

export default App;
