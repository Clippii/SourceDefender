package service

import (
    "bytes"
    "encoding/csv"
    "errors"
    "fmt"
    "net/url"
    "regexp"
    "strconv"
    "strings"
)

func validTax(rate int) bool { return rate >= 0 && rate <= 10000 }
func invalid(message string) error { return errors.New(message) }
func reference(id string) string { return "invoice-" + id }
func mean(values []float64) float64 { var total float64; for _, v := range values { total += v }; if len(values)==0{return 0};return total/float64(len(values)) }
func validDigest(value string) bool { return regexp.MustCompile(`^[a-f0-9]{64}$`).MatchString(value) }
func validFeed(value string) bool { u, err := url.Parse(value); return err == nil && u.Scheme == "https" && u.Host != "" }

func compareVersions(left, right string) (int, error) {
    parse := func(value string) ([]int, error) {
        parts := strings.Split(value, "."); if len(parts) != 3 { return nil, errors.New("expected major.minor.patch") }
        result := []int{}
        for _, part := range parts { n, err := strconv.Atoi(part); if err != nil || n < 0 { return nil, errors.New("invalid version") };result = append(result, n) }; return result, nil
    }
    a, err := parse(left); if err != nil { return 0, err }; b, err := parse(right); if err != nil { return 0, err }
    for i := range a { if a[i] < b[i] { return -1, nil };if a[i] > b[i] { return 1, nil } };return 0, nil
}

func encodeCSV(records []Record) (string, error) {
    var buffer bytes.Buffer; writer := csv.NewWriter(&buffer)
    if err := writer.Write([]string{"id", "kind", "owner", "title", "status", "amount"}); err != nil { return "", err }
    for _, row := range records {
        title := row.Title
        if len(title) > 0 && strings.ContainsRune("=+-@", rune(title[0])) { title = "'" + title }
        if err := writer.Write([]string{row.ID,row.Kind,row.Owner,title,row.Status,fmt.Sprint(row.Amount)}); err != nil { return "", err }
    }
    writer.Flush();return buffer.String(), writer.Error()
}

func normalizeTitles(records []Record) []string {
    result := []string{};for _, r := range records { result = append(result,strings.ToLower(strings.TrimSpace(r.Title))) };return result
}
