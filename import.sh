#!/bin/sh

curl https://vsza.hu/hacksense/history.csv | tail -n +2 | sqlite3 --init import.sql hacksense.sqlite3
