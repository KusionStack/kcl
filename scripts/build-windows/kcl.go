// Copyright 2021 The KCL Authors. All rights reserved.

//go:build ingore && windows
// +build ingore,windows

package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"unsafe"
)

func main() {
	// kclvm -m kclvm ...
	inputPath, err := os.Executable()
	if err != nil {
		fmt.Println("Input path does not exist")
		os.Exit(1)
	}

	parentPath := filepath.Dir(inputPath)

	install_kclvm(parentPath)
	var args []string
	args = append(args, os.Args[0])
	args = append(args, "-m", "kclvm")
	args = append(args, os.Args[1:]...)
	os.Exit(Py_Main(args))
}

var (
	python39_dll = syscall.NewLazyDLL(findKclvm_dllPath())
	proc_Py_Main = python39_dll.NewProc("Py_Main")
)

// int Py_Main(int argc, wchar_t **argv)
func Py_Main(args []string) int {
	c_args := make([]*uint16, len(args)+1)
	for i, s := range args {
		c_args[i] = syscall.StringToUTF16Ptr(s)
	}
	ret, _, _ := proc_Py_Main.Call(uintptr(len(args)), uintptr(unsafe.Pointer(&c_args[0])))
	return int(ret)
}

func findKclvm_dllPath() string {
	kclvmName := "python39.dll"

	if exePath, _ := os.Executable(); exePath != "" {
		exeDir := filepath.Join(exePath, "..", "..")
		if fi, _ := os.Stat(filepath.Join(exeDir, kclvmName)); fi != nil && !fi.IsDir() {
			return filepath.Join(exeDir, kclvmName)
		}
	}
	if wd, _ := os.Getwd(); wd != "" {
		if fi, _ := os.Stat(filepath.Join(wd, kclvmName)); fi != nil && !fi.IsDir() {
			return filepath.Join(wd, kclvmName)
		}
		wd = filepath.Join(wd, "_output/kclvm-windows")
		if fi, _ := os.Stat(filepath.Join(wd, kclvmName)); fi != nil && !fi.IsDir() {
			return filepath.Join(wd, kclvmName)
		}
	}

	return kclvmName
}

func install_kclvm(installed_path string) {
	// Check if Python3 is installed
	cmd := exec.Command("cmd", "/C", "where python3")
	_, err := cmd.Output()
	if err != nil {
		fmt.Println("Python3 is not installed, details: ", err)
		os.Exit(1)
	}
	// Check if "installed" file exists

	outputPath := filepath.Join(installed_path, "kclvm_installed")
	if _, err := os.Stat(outputPath); err == nil {
		return
	}

	// Install kclvm module using pip
	cmd = exec.Command("cmd", "/C", "python3", "-m", "pip", "install", "kclvm")

	_, err = cmd.CombinedOutput()
	if err != nil {
		fmt.Println("Pip install kclvm falied ", err)
		os.Exit(1)
	}

	// Create "installed" file
	f, err := os.Create(outputPath)
	if err != nil {
		fmt.Printf("Error creating file: %s\n", err)
		os.Exit(1)
	}
	defer f.Close()
}
