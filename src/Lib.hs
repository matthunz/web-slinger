module Lib
    ( someFunc
    ) where
import Foreign.C (CString)
import Foreign (FunPtr)

someFunc :: IO ()
someFunc = putStrLn "someFunc"

foreign import ccall "wrapper"
  wrap :: (CString -> IO ()) -> IO (FunPtr (CString -> IO ()))

foreign import ccall "c_start" start :: FunPtr (CString -> IO ()) -> IO ()

foreign import ccall unsafe "c_eval" evalJs :: CString -> IO ()
