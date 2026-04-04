def csv_scan_cost(rows: int, cols: int) -> int:
    checksum = 0
    for row in range(rows):
        for col in range(cols):
            field = ((row * 37) + (col * 13)) % 1009
            if (col + 1) < cols:
                checksum += field + 44
            else:
                checksum += field + 10
    return checksum


if __name__ == "__main__":
    print(csv_scan_cost(900000, 9))
