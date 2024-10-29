#!/bin/bash
# common parameters
TYPE_LIST=(read write randread randwrite readwrite randrw)
THREAD_LIST=(1 2 6)
BS_LIST=(4k 32k 128k 1024k 16384k)
RUN_TIME=10s
BLK_LIST=(nullb0 rnullb0)

CLASS=$1

TYPE_C="C"
TYPE_LKM="LKM"
TYPE_DM="DM"


run_fio(){
  type=$1
  thread=$2
  bs=$3
  echo Run command: sudo fio --name=mytest --ioengine=psync --rw=${type} --bs=${bs} --numjobs=${thread} --time_based --runtime=${RUN_TIME} -filename=/dev/${CLASS} --size=4G -group_reporting -direct=1
  count=1
  while(( count<=3 ))
  do
    echo ----------TEST Impl:${CLASS} TYPE:$type THREAD:$thread BS:$bs $count start----------
    sudo fio --name=mytest --ioengine=psync --rw=${type} --bs=${bs} --numjobs=${thread} --time_based --runtime=${RUN_TIME} -filename=/dev/${CLASS} --size=4G -group_reporting -direct=1
    sync
    sleep 2
    echo ----------TEST Impl:${CLASS} TYPE:$type THREAD:$thread BS:$bs $count over ----------
    ((count++))
  done
  return 0
}


run_one_blk(){
 # shellcheck disable=SC2068
 for bs in ${BS_LIST[@]}
 do
   for thread in ${THREAD_LIST[@]}
   do
     for type in ${TYPE_LIST[@]}
     do
       run_fio "$type" "$thread" "$bs"
     done
   done
 done
}


check_blk(){
  if [ -b /dev/${CLASS} ]
  then
  	echo /dev/${CLASS}  is a block device
  else
  	echo /dev/${CLASS} is not a block device, exit
  	exit
  fi
}

run_all(){
  # shellcheck disable=SC2068
  for blk in ${BLK_LIST[@]}
  do
    CLASS=$blk
    check_blk
    run_one_blk
  done
}


if [[ $CLASS = $TYPE_LKM || $CLASS = $TYPE_DM ]]
then
	CLASS=rnullb0
elif [ $CLASS = $TYPE_C ]
  then
    CLASS=nullb0
elif [ $CLASS = "all" ]
then
  run_all
else
  echo "Usage: $0 [C|LKM|DM|all]"
  exit
fi

if [ $CLASS != "all" ]
then
  check_blk
  run_one_blk
else
  run_all
fi
