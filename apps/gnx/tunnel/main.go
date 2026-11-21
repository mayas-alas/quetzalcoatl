package main

import (
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/tailscale/tailcat"
)

func main() {
	if err := relay(); err != nil {
		// Only fixed messages from relay are printed, never upstream errors,
		// which can include the invitation or its DERP metadata.
		fmt.Fprintln(os.Stderr, "gnx:", err)
		os.Exit(1)
	}
}

func invitation() ([]byte, error) {
	const prefix = `\\.\pipe\GnX.Connect.`
	if len(os.Args) != 2 {
		return nil, errors.New("canal local de invitación ausente")
	}
	id, ok := strings.CutPrefix(os.Args[1], prefix)
	if !ok || len(id) != 32 || strings.ContainsFunc(id, func(r rune) bool {
		return !strings.ContainsRune("0123456789abcdef", r)
	}) {
		return nil, errors.New("canal local de invitación inválido")
	}
	pipe, err := os.OpenFile(os.Args[1], os.O_RDWR, 0)
	if err != nil {
		return nil, errors.New("no se pudo abrir el canal local de invitación")
	}
	defer pipe.Close()
	var size [2]byte
	if _, err := io.ReadFull(pipe, size[:]); err != nil {
		return nil, errors.New("no se recibió la invitación")
	}
	n := binary.LittleEndian.Uint16(size[:])
	if n < 3 || n > 1024 {
		return nil, errors.New("tamaño de invitación inválido")
	}
	invite := make([]byte, n)
	if _, err := io.ReadFull(pipe, invite); err != nil {
		clear(invite)
		return nil, errors.New("invitación incompleta")
	}
	if _, err := pipe.Write([]byte{1}); err != nil {
		clear(invite)
		return nil, errors.New("no se pudo confirmar la recepción")
	}
	return invite, nil
}

func relay() error {
	invite, err := invitation()
	if err != nil {
		return err
	}
	addr := tailcat.Addr(string(invite))
	clear(invite)
	info, err := tailcat.ParseAddr(addr)
	if err != nil || info.ServerPublic.IsZero() || info.PresharedKey.IsZero() ||
		info.RegionID == 0 || len(info.Region) != 0 {
		return errors.New("se requiere una invitación Tailcat con clave precompartida y relay predeterminado")
	}
	client := &tailcat.Client{Server: addr, Logf: func(string, ...any) {}}
	defer client.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 25*time.Second)
	conn, err := client.DialTCPPort(ctx, 22)
	cancel()
	if err != nil {
		return errors.New("la VM no responde por Tailcat; verifica la invitación y la ventana de acceso")
	}
	defer conn.Close()

	// OpenSSH owns the terminal and the SSH handshake. This process only
	// bridges its byte streams, with no local TCP listener.
	done := make(chan error, 1)
	go func() {
		_, err := io.Copy(conn, os.Stdin)
		done <- err
		_ = conn.Close()
	}()
	if _, err := io.Copy(os.Stdout, conn); err != nil {
		select {
		case <-done:
			return nil
		default:
			return errors.New("se interrumpió el transporte Tailcat")
		}
	}
	return nil
}
